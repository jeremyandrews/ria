use std::path::Path;
use std::str::FromStr;

use file_format::FileFormat;
use gstreamer_pbutils::DiscovererAudioInfo;
use gstreamer_pbutils::{prelude::*, DiscovererContainerInfo};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sea_orm::*;
use tracing::{event, instrument, Level};
use walkdir::WalkDir;

use crate::database;
use crate::entities::{prelude::*, *};
use crate::musicbrainz;
use crate::tags;
use crate::utils;

/// The general media types Ria works with.
pub(crate) enum MediaType {
    /// Audio files, such as FLAC or MP3.
    Audio,
    /// Image files, such as JPG.
    Image,
    /// Text files, such as CUE.
    _Text,
    /// Everything else (including unsupported audio and image formats).
    Unknown,
}
impl FromStr for MediaType {
    // @TODO: At this time no error is returned.
    type Err = anyhow::Error;

    #[instrument]
    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        event!(Level::TRACE, "from_str");
        if s.starts_with("audio/") {
            Ok(MediaType::Audio)
        } else if s.starts_with("image/") {
            Ok(MediaType::Image)
        // @TODO: Library is returning "application/octet-stream" for "text" files.
        } else {
            Ok(MediaType::Unknown)
        }
    }
}

pub(crate) async fn store_audio_artist(audio_id: i32, artist_id: i32) {
    let audio_artist = audio_artist::ActiveModel {
        audio_id: ActiveValue::Set(audio_id),
        artist_id: ActiveValue::Set(artist_id),
        ..Default::default()
    };

    event!(Level::DEBUG, "Insert AudioArtist: {:?}", audio_artist);
    let db = database::connection().await;
    AudioArtist::insert(audio_artist)
        .exec(db)
        .await
        .expect("failed to write audio_artist to database");
}

// Map directory to all artists found in contained audio files.
pub(crate) async fn store_artist_directory(audio_id: i32, artist_id: i32) {
    let now = chrono::Utc::now().naive_utc();

    let db = database::connection().await;

    // Build enum to select directory_id column into a Vec<i32>.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
    enum QueryAs {
        DirectoryId,
    }
    let results: Vec<i32> = match audio_directory::Entity::find()
        .filter(audio_directory::Column::AudioId.eq(audio_id))
        .select_only()
        .column_as(audio_directory::Column::DirectoryId, QueryAs::DirectoryId)
        .into_values::<_, QueryAs>()
        .all(db)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            event!(Level::WARN, "Audio::find() directory_id failure: {}", e);
            // @TODO: What to do?
            return;
        }
    };
    for directory_id in results {
        // @TODO Check if already existing.
        let existing = {
            match ArtistDirectory::find()
                .filter(artist_directory::Column::ArtistId.eq(artist_id))
                .filter(artist_directory::Column::DirectoryId.eq(directory_id))
                .one(db)
                .await
            {
                Ok(e) => e,
                Err(e) => {
                    event!(Level::WARN, "Audio::find() failure: {}", e);
                    break;
                }
            }
        };

        // Only add if not already existing.
        if existing.is_none() {
            let artist_directory = artist_directory::ActiveModel {
                created: ActiveValue::Set(now.to_owned()),
                updated: ActiveValue::Set(now.to_owned()),
                artist_id: ActiveValue::Set(artist_id),
                directory_id: ActiveValue::Set(directory_id),
                ..Default::default()
            };

            event!(
                Level::DEBUG,
                "Insert ArtistDirectory: {:?}",
                artist_directory
            );
            ArtistDirectory::insert(artist_directory)
                .exec(db)
                .await
                .expect("failed to write artist_directory to database");
        }
    }
}

// Scan for media files.
pub(crate) async fn scan_media_files(path: &str) {
    // Percent-encode all characters except alpha-numerics and "/" to build proper
    // paths. @TODO: remove characters necessary to navigate Windows paths.
    const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/');

    let walker = WalkDir::new(path).follow_links(true).into_iter();
    for (counter, entry) in walker.filter_entry(|e| !utils::is_hidden(e)).enumerate() {
        let metadata = match entry.as_ref() {
            Ok(i) => match i.metadata() {
                Ok(m) => m,
                Err(e) => {
                    event!(Level::WARN, "metadata() failure: {}", e);
                    continue;
                }
            },
            Err(e) => {
                event!(Level::WARN, "WalkDir entry.as_ref() failure: {}", e);
                continue;
            }
        };

        // Files may be tracks, images, playlists, and more.
        if metadata.is_file() {
            let format = match FileFormat::from_file(match entry.as_ref() {
                Ok(i) => i.path(),
                Err(e) => {
                    event!(Level::WARN, "WalkDir entry.as_ref() failure: {}", e);
                    continue;
                }
            }) {
                Ok(f) => f,
                Err(e) => {
                    event!(Level::WARN, "FileFormat::from_file() failure: {}", e);
                    continue;
                }
            };

            let media_type = MediaType::from_str(format.media_type()).unwrap_or(MediaType::Unknown);
            match media_type {
                MediaType::Image => {
                    // @TODO: Associate the image with the album or artist depending on where it is.
                    event!(
                        Level::DEBUG,
                        "Image detected ({}): {}",
                        format.media_type(),
                        match entry.as_ref() {
                            Ok(d) => d.path().display(),
                            Err(e) => {
                                event!(Level::WARN, "WalkDir entry.as_ref() failure: {}", e);
                                continue;
                            }
                        }
                    );
                }
                MediaType::_Text => {
                    // @TODO: How to properly detect text?
                }
                MediaType::Audio => {
                    // Store any tags that are found in the audio file.
                    let mut tags = None;

                    // Build an absolute URI as required by GStreamer.
                    let path = Path::new(match &std::env::current_dir() {
                        Ok(c) => c,
                        Err(e) => {
                            event!(Level::WARN, "std::env::current_dir() failure: {}", e);
                            continue;
                        }
                    })
                    .join(match entry.as_ref() {
                        Ok(d) => d.path(),
                        Err(e) => {
                            event!(Level::WARN, "WalkDir entry.as_ref() failure: {}", e);
                            continue;
                        }
                    });

                    let uri = format!(
                        "file://{}",
                        utf8_percent_encode(
                            match path.to_str() {
                                Some(p) => p,
                                None => {
                                    event!(Level::WARN, "path.to_str() failure: NONE");
                                    continue;
                                }
                            },
                            FRAGMENT
                        )
                        .collect::<String>(),
                    );

                    event!(
                        Level::DEBUG,
                        "Audio file detected at path: {} ({})",
                        path.display(),
                        uri
                    );

                    // Check if this audio file is already in the database.
                    let existing = {
                        let db = database::connection().await;
                        match Audio::find()
                            .filter(audio::Column::Uri.contains(&uri))
                            .one(db)
                            .await
                        {
                            Ok(e) => e,
                            Err(e) => {
                                event!(Level::WARN, "Audio::find() failure: {}", e);
                                break;
                            }
                        }
                    };

                    // @TODO: compare to database.
                    let mut audio = audio::ActiveModel {
                        uri: ActiveValue::Set(uri.clone()),
                        path: ActiveValue::Set(match path.parent() {
                            Some(p) => p.display().to_string(),
                            None => {
                                event!(Level::WARN, "path.parent() returned none");
                                "".to_string()
                            }
                        }),
                        name: ActiveValue::Set(match path.file_name() {
                            Some(f) => f.to_str().unwrap_or("").to_string(),
                            None => {
                                event!(Level::WARN, "path.file_name() returned nothing");
                                "".to_string()
                            }
                        }),
                        extension: ActiveValue::Set(match path.extension() {
                            Some(e) => e.to_str().unwrap_or("").to_string(),
                            None => {
                                event!(Level::WARN, "path.extension() returned nothing");
                                "".to_string()
                            }
                        }),
                        // The following values will be replaced later if GStreamer is able to
                        // identify the contents of this audio file.
                        format: ActiveValue::Set("UNKNOWN".to_string()),
                        duration: ActiveValue::Set(0),
                        channels: ActiveValue::Set(0),
                        bits: ActiveValue::Set(0),
                        hertz: ActiveValue::Set(0),
                        ..Default::default()
                    };

                    let timeout: gstreamer::ClockTime = gstreamer::ClockTime::from_seconds(15);
                    let discoverer = match gstreamer_pbutils::Discoverer::new(timeout) {
                        Ok(d) => d,
                        Err(e) => {
                            event!(Level::WARN, "Discoverer::new() failure: {}", e);
                            continue;
                        }
                    };
                    let info = match discoverer.discover_uri(&uri) {
                        Ok(u) => u,
                        Err(e) => {
                            event!(Level::WARN, "discover_uri({}) failure: {}", uri, e);
                            continue;
                        }
                    };

                    event!(
                        Level::DEBUG,
                        "Duration: {}",
                        info.duration().unwrap_or_else(|| gstreamer::ClockTime::NONE
                            .expect("failed to create empty ClockTime"))
                    );
                    audio.duration = sea_orm::ActiveValue::Set(match info.duration() {
                        Some(d) => match d.seconds().try_into() {
                            Ok(s) => s,
                            Err(e) => {
                                event!(Level::WARN, "mseconds.try_into() failure: {}", e);
                                continue;
                            }
                        },
                        None => {
                            event!(Level::WARN, "info.duration() returned nothing");
                            continue;
                        }
                    });

                    if let Some(stream_info) = info.stream_info() {
                        let caps_str = if let Some(caps) = stream_info.caps() {
                            if caps.is_fixed() {
                                gstreamer_pbutils::pb_utils_get_codec_description(&caps)
                                    .unwrap_or_else(|_| glib::GString::from("unknown codec"))
                            } else {
                                glib::GString::from(caps.to_string())
                            }
                        } else {
                            glib::GString::from("")
                        };
                        audio.format = sea_orm::ActiveValue::Set(caps_str.to_string().to_owned());

                        if let Some(container_info) =
                            stream_info.downcast_ref::<DiscovererContainerInfo>()
                        {
                            event!(
                                Level::WARN,
                                "@TODO @@@@@@@@@@: Handle containers... {:#?}",
                                container_info
                            );
                        } else if let Some(container_audio) =
                            stream_info.downcast_ref::<DiscovererAudioInfo>()
                        {
                            audio.channels = sea_orm::ActiveValue::Set(
                                container_audio
                                    .channels()
                                    .try_into()
                                    .expect("failed to convert u32 to i32"),
                            );
                            audio.bits = sea_orm::ActiveValue::Set(
                                container_audio
                                    .depth()
                                    .try_into()
                                    .expect("failed to convert u32 to i32"),
                            );
                            audio.hertz = sea_orm::ActiveValue::Set(
                                container_audio
                                    .sample_rate()
                                    .try_into()
                                    .expect("failed to convert u32 to i32"),
                            );
                            // @TODO: explore if there's any value in the following fields:
                            //   * container_audio.bitrate()
                            //   * container_audio.max_bitrate()
                            //   * container_audio.language()

                            // Store any tags that may be in the audio file.
                            tags = info.tags();
                        } else {
                            event!(Level::WARN, "@TODO @@@@@@@@@@: Handle non-audio streams");
                        }
                    }
                    // @TODO: Detect changes to the files, and update as needed.
                    // @TODO: Error handling.
                    if existing.is_none() {
                        event!(Level::DEBUG, "Insert Audio File: {:?}", audio);
                        let new_audio = {
                            let db = database::connection().await;
                            Audio::insert(audio)
                                .exec(db)
                                .await
                                .expect("failed to write audio details to database")
                        };

                        if let Some(tags) = tags {
                            for (name, values) in tags.iter_generic() {
                                event!(Level::DEBUG, "tag {}: {:?}", name, values);
                                let values = tags::get_tags(name, values);
                                for value in values {
                                    let tag = audio_tag::ActiveModel {
                                        audio_id: ActiveValue::Set(new_audio.last_insert_id),
                                        name: ActiveValue::Set(name.to_string()),
                                        value: ActiveValue::Set(value.to_string()),
                                        ..Default::default()
                                    };
                                    {
                                        event!(Level::DEBUG, "Insert AudioTag: {:?}", tag);
                                        let db = database::connection().await;
                                        AudioTag::insert(tag)
                                            .exec(db)
                                            .await
                                            .expect("failed to write tag to database");
                                    }
                                    if name == "artist" {
                                        let existing_artist = {
                                            let db = database::connection().await;
                                            match Artist::find()
                                                .filter(artist::Column::Name.contains(&value))
                                                .one(db)
                                                .await
                                            {
                                                Ok(e) => e,
                                                Err(e) => {
                                                    event!(
                                                        Level::WARN,
                                                        "Artist::find() failure: {}",
                                                        e
                                                    );
                                                    break;
                                                }
                                            }
                                        };

                                        if let Some(artist) = existing_artist {
                                            store_audio_artist(
                                                new_audio.last_insert_id,
                                                artist.artist_id,
                                            )
                                            .await;
                                        } else {
                                            // Artist doesn't exist in our database, add to MusicBrainz queue
                                            // to download details.
                                            musicbrainz::add_to_queue(musicbrainz::QueuePayload {
                                                payload_type: musicbrainz::PayloadType::AudioArtist,
                                                id: new_audio.last_insert_id,
                                                value: value.to_string(),
                                            })
                                            .await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                MediaType::Unknown => {
                    // @TODO: Deal with audio files that we didn't properly detect.
                    // @TODO: Perhaps detect text files in a second pass here, on the file extension?
                    event!(
                        Level::WARN,
                        "UNKNOWN ({}): {}",
                        format.media_type(),
                        match entry.as_ref() {
                            Ok(d) => d.path().display(),
                            Err(e) => {
                                event!(Level::WARN, "WalkDir entry.as_ref() failure: {}", e);
                                continue;
                            }
                        }
                    );
                }
            }
        // Albums are collected together in directories.
        } else if metadata.is_dir() {
            // @TODO: Track directories for visualization, organization, and to assist in
            // auto-identifying albums.
        }

        // @TODO: Make this configurable and optional.
        if counter > 25_000 {
            break;
        }
    }

    // Next, group audio files into directories (giving an initial view of what are most likely albums).
    {
        let db = database::connection().await;
        // Build enum to select column into a Vec<String>.
        #[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
        enum QueryAs {
            GroupByPath,
        }
        let results: Vec<String> = match audio::Entity::find()
            .select_only()
            .column_as(audio::Column::Path, QueryAs::GroupByPath)
            .group_by(audio::Column::Path)
            .into_values::<_, QueryAs>()
            .all(db)
            .await
        {
            Ok(q) => q,
            Err(e) => {
                event!(Level::WARN, "Audio::find() directories failure: {}", e);
                // @TODO: What to do?
                return;
            }
        };
        for result in results {
            let components = Path::new(&result).components();
            // @TODO: Error handling.
            let name = components.last().unwrap();

            let now = chrono::Utc::now().naive_utc();
            let new_directory = directory::ActiveModel {
                created: ActiveValue::Set(now.to_owned()),
                updated: ActiveValue::Set(now.to_owned()),
                // @TODO: Error handling.
                name: ActiveValue::Set(name.as_os_str().to_str().unwrap().to_owned()),
                path: ActiveValue::Set(result.to_owned()),
                ..Default::default()
            };
            let created_directory = Directory::insert(new_directory)
                .exec(db)
                .await
                .expect("failed to write directory to database");

            // Find all audio files contained in the directory.
            let audio_files = match Audio::find()
                .filter(audio::Column::Path.contains(&result))
                .all(db)
                .await
            {
                Ok(e) => e,
                Err(e) => {
                    event!(Level::WARN, "Artist::find() by path failure: {}", e);
                    // @TODO: what to do?
                    return;
                }
            };
            for audio_file in audio_files {
                let audio = audio_directory::ActiveModel {
                    created: ActiveValue::Set(now.to_owned()),
                    updated: ActiveValue::Set(now),
                    directory_id: ActiveValue::Set(created_directory.last_insert_id),
                    audio_id: ActiveValue::Set(audio_file.audio_id),
                    ..Default::default()
                };
                AudioDirectory::insert(audio)
                    .exec(db)
                    .await
                    .expect("failed to write audio_directory to database");
            }
        }

        // At this point individual albums can be listed as follows:
        // SELECT d.name, a.name FROM audio_directory AS ad LEFT JOIN audio AS a ON ad.audio_id = a.audio_id LEFT JOIN directory AS d ON ad.directory_id = d.directory_id WHERE ad.directory_id = 7 ORDER BY a.name

        // And to include the artist name in the list (if identified):
        // SELECT ar.name, d.name, a.name FROM audio_directory AS ad LEFT JOIN audio AS a ON ad.audio_id = a.audio_id LEFT JOIN directory AS d ON ad.directory_id = d.directory_id LEFT JOIN artist_directory AS ard ON ard.directory_id = d.directory_id LEFT JOIN artist AS ar ON ard.artist_id = ar.artist_id WHERE ad.directory_id = 13 ORDER BY a.name;
    }
}
