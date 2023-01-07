use std::path::Path;
use std::str::FromStr;

use file_format::FileFormat;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sea_orm::*;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::{event, instrument, Level};
use walkdir::WalkDir;

use crate::database;
use crate::entities::{prelude::*, *};
use crate::musicbrainz;
use crate::utils;
use crate::Config;

#[derive(Clone, Debug, FromQueryResult)]
pub(crate) struct MediaList {
    pub(crate) audio_name: String,
    pub(crate) audio_path: String,
    pub(crate) audio_id: u32,
    pub(crate) directory_name: String,
    pub(crate) artist_name: Option<String>,
}

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

pub(crate) async fn store_audio_artist(config: &Config, audio_id: i32, artist_id: i32) {
    let audio_artist = audio_artist::ActiveModel {
        audio_id: ActiveValue::Set(audio_id),
        artist_id: ActiveValue::Set(artist_id),
        ..Default::default()
    };

    event!(Level::DEBUG, "Insert AudioArtist: {:?}", audio_artist);
    let db = database::connection(config).await;
    AudioArtist::insert(audio_artist)
        .exec(db)
        .await
        .expect("failed to write audio_artist to database");
}

// Map directory to all artists found in contained audio files.
pub(crate) async fn store_artist_directory(config: &Config, audio_id: i32, artist_id: i32) {
    event!(
        Level::TRACE,
        "store_artist_directory audio_id({}) artist_id({})",
        audio_id,
        artist_id
    );
    let now = chrono::Utc::now().naive_utc();

    let db = database::connection(config).await;

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

// @TODO: Optional filters (ie, artist, etc)
pub(crate) async fn print_media(config: &Config) {
    let media = get_media(config).await;
    let mut last_artist = None;
    let mut last_album = String::new();
    for audio in media {
        if !audio.artist_name.eq(&last_artist) {
            last_artist = audio.artist_name;
            if let Some(artist) = last_artist.as_ref() {
                println!("\n{}:", artist)
            } else {
                println!("\nUnidentified artist:");
            }
        }
        if last_album != audio.directory_name {
            last_album = audio.directory_name;
            println!("  {}", last_album)
        }
        println!("     - {}", audio.audio_name);
    }
}

// @TODO: Optional filters (ie, artist, etc)
pub(crate) async fn get_media(config: &Config) -> Vec<MediaList> {
    let db = database::connection(config).await;
    // SELECT ar.name, d.name, a.name FROM audio_directory AS ad
    //   LEFT JOIN audio AS a ON ad.audio_id = a.audio_id
    //   LEFT JOIN directory AS d ON ad.directory_id = d.directory_id
    //   LEFT JOIN artist_directory AS ard ON ard.directory_id = d.directory_id
    //   LEFT JOIN artist AS ar ON ard.artist_id = ar.artist_id
    // ORDER BY a.name;

    //match audio_directory::Entity::find()
    let mut select_query = audio_directory::Entity::find()
        .left_join(Audio)
        .left_join(Directory)
        .join(
            JoinType::LeftJoin,
            directory::Relation::ArtistDirectory.def(),
        )
        .join(JoinType::LeftJoin, artist_directory::Relation::Artist.def())
        .select_only()
        .column_as(audio::Column::AudioId, "audio_id")
        .column_as(audio::Column::Path, "audio_path")
        .column_as(audio::Column::Name, "audio_name")
        .column_as(directory::Column::Name, "directory_name")
        .column_as(artist::Column::Name, "artist_name");

    if config.artist.is_some() {
        select_query =
            select_query.filter(artist::Column::Name.contains(&config.artist.as_ref().unwrap()));
    }

    if config.directory.is_some() {
        select_query = select_query
            .filter(directory::Column::Name.contains(&config.directory.as_ref().unwrap()));
    }

    if config.track.is_some() {
        select_query =
            select_query.filter(audio::Column::Name.contains(&config.track.as_ref().unwrap()));
    }

    match select_query
        .order_by_asc(artist::Column::SortName)
        .order_by_asc(directory::Column::Name)
        .order_by_asc(audio::Column::Name)
        .into_model::<MediaList>()
        .all(db)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // No results to return.
            event!(
                Level::WARN,
                "AudioDirectory::find() list_media failure: {}",
                e
            );
            Vec::new()
        }
    }
}

// Scan for media files.
pub(crate) async fn scan_media_files(config: &Config) {
    // Percent-encode all characters except alpha-numerics and "/" to build proper
    // paths. @TODO: remove characters necessary to navigate Windows paths.
    const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/');

    let path = config
        .library
        .as_ref()
        .expect("library must exist")
        .to_string();
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
                    // Build an absolute URI to uniquely identify files.
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
                        let db = database::connection(config).await;
                        match Audio::find()
                            .filter(audio::Column::Uri.like(&uri))
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

                    let extension = match path.extension() {
                        Some(e) => e.to_str().unwrap_or(""),
                        None => {
                            event!(Level::WARN, "path.extension() returned nothing");
                            ""
                        }
                    };

                    // @TODO: compare to database.
                    let mut audio = audio::ActiveModel {
                        uri: ActiveValue::Set(uri.clone()),
                        path: ActiveValue::Set(match entry.as_ref().unwrap().path().parent() {
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
                        extension: ActiveValue::Set(extension.to_string()),
                        // The following values will be replaced later if Symphonia is able to
                        // identify the contents of this audio file.
                        format: ActiveValue::Set("UNKNOWN".to_string()),
                        duration: ActiveValue::Set(0),
                        channels: ActiveValue::Set(0),
                        bits: ActiveValue::Set(0),
                        hertz: ActiveValue::Set(0),
                        ..Default::default()
                    };

                    let src = std::fs::File::open(&path).expect("failed to open media");
                    let mss = MediaSourceStream::new(Box::new(src), Default::default());

                    // Add file suffix hint to speed of probe.
                    let mut hint = Hint::new();
                    if !extension.is_empty() {
                        hint.with_extension(extension);
                    }

                    // Use the default options for metadata and format readers.
                    let meta_opts: MetadataOptions = Default::default();
                    let fmt_opts: FormatOptions = Default::default();

                    // Probe the media source.
                    let mut probed = match symphonia::default::get_probe()
                        .format(&hint, mss, &fmt_opts, &meta_opts)
                    {
                        Ok(p) => p,
                        Err(e) => {
                            event!(Level::WARN, "Symphonia get_probe() failure: {}", e);
                            continue;
                        }
                    };

                    let tracks = probed.format.tracks();
                    for (idx, track) in tracks.iter().enumerate() {
                        assert!(idx == 0);
                        let params = &track.codec_params;

                        if let Some(codec) =
                            symphonia::default::get_codecs().get_codec(params.codec)
                        {
                            audio.format = sea_orm::ActiveValue::Set(codec.long_name.to_string());
                        }

                        // Get duration.
                        if let Some(n_frames) = params.n_frames {
                            if let Some(tb) = params.time_base {
                                audio.duration = sea_orm::ActiveValue::Set(
                                    tb.calc_time(n_frames).seconds as i32,
                                );
                            }
                        }
                        // Get channels.
                        audio.channels =
                            sea_orm::ActiveValue::Set(params.channels.unwrap().count() as i32);
                        audio.bits = sea_orm::ActiveValue::Set(match params.bits_per_sample {
                            Some(b) => b as i32,
                            None => 0,
                        });
                        audio.hertz = sea_orm::ActiveValue::Set(params.sample_rate.unwrap() as i32);
                    }

                    // @TODO: Detect changes to the files, and update as needed.
                    // @TODO: Error handling.
                    if existing.is_none() {
                        event!(Level::DEBUG, "Insert Audio File: {:?}", audio);
                        let new_audio = {
                            let db = database::connection(config).await;
                            Audio::insert(audio)
                                .exec(db)
                                .await
                                .expect("failed to write audio details to database")
                        };

                        if let Some(metadata_rev) = probed.format.metadata().current() {
                            // Step through only known tags.
                            for tag in metadata_rev.tags().iter().filter(|tag| tag.is_known()) {
                                if let Some(std_key) = tag.std_key {
                                    event!(Level::DEBUG, "tag {:?}: {}", std_key, tag.value);
                                    let name = format!("{:?}", std_key);
                                    let new_tag = audio_tag::ActiveModel {
                                        audio_id: ActiveValue::Set(new_audio.last_insert_id),
                                        name: ActiveValue::Set(name.to_string()),
                                        value: ActiveValue::Set(tag.value.to_string()),
                                        ..Default::default()
                                    };
                                    {
                                        event!(Level::DEBUG, "Insert AudioTag: {:?}", new_tag);
                                        let db = database::connection(config).await;
                                        AudioTag::insert(new_tag)
                                            .exec(db)
                                            .await
                                            .expect("failed to write tag to database");
                                    }
                                    if name == "Artist" {
                                        let existing_artist = {
                                            let db = database::connection(config).await;
                                            match Artist::find()
                                                .filter(
                                                    artist::Column::Name
                                                        .like(&tag.value.to_string()),
                                                )
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
                                                config,
                                                new_audio.last_insert_id,
                                                artist.artist_id,
                                            )
                                            .await;
                                        } else {
                                            // Artist doesn't exist in our database, add to MusicBrainz queue
                                            // to download details.
                                            musicbrainz::add_to_queue(
                                                config,
                                                musicbrainz::QueuePayload {
                                                    payload_type:
                                                        musicbrainz::PayloadType::AudioArtist,
                                                    id: new_audio.last_insert_id,
                                                    value: tag.value.to_string(),
                                                },
                                            )
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
        let db = database::connection(config).await;
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
                .filter(audio::Column::Path.like(&result))
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
    }
}
