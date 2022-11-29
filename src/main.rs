mod database;
mod entities;
mod tags;

use std::path::Path;
use std::str::FromStr;

use file_format::FileFormat;
use gstreamer_pbutils::DiscovererAudioInfo;
use gstreamer_pbutils::{prelude::*, DiscovererContainerInfo};
use musicbrainz_rs::Search;
use once_cell::sync::OnceCell;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sea_orm::*;
use tracing::{event, instrument, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use walkdir::{DirEntry, WalkDir};

use crate::database::{RiaArtistType, RiaGender};

use entities::{prelude::*, *};

static USER_AGENT: OnceCell<String> = OnceCell::new();

/// The general media types Ria works with.
enum MediaType {
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

/// Skip files and directories that start with ".".
#[instrument]
fn is_hidden(entry: &DirEntry) -> bool {
    event!(Level::TRACE, "is_hidden");
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Log all events to a file. @TODO: add configurable rolling file support.
    let logfile = RollingFileAppender::new(Rotation::NEVER, "./", "ria.log");
    // Log `INFO` and above to stdout.
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_writer(stdout.and(logfile))
        .init();

    // Initialize the database. @TODO: Error handling.
    let db = database::connection()
        .await
        .expect("failed to connect to database");

    // Initialize GStreamer.
    gstreamer::init().expect("failed to initialize gstreamer");

    // Percent-encode all characters except alpha-numerics and "/" to build proper
    // paths. @TODO: remove characters necessary to navigate Windows paths.
    const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/');

    // Dynamically build a user agent from package name and package version. Store
    // in a OnceCell to allow static lifetime necessary for the MusicBrainz agent.
    USER_AGENT
        .set(format!(
            "{}/{} (https://github.com/jeremyandrews/ria)",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .expect("failed to set USER_AGENT");

    // Set up the MusicBrainz agent.
    musicbrainz_rs::config::set_user_agent(USER_AGENT.get().expect("failed to get USER_AGENT"));

    // @TODO: Make directories configurable.
    let walker = WalkDir::new("music").follow_links(true).into_iter();
    for (counter, entry) in walker.filter_entry(|e| !is_hidden(e)).enumerate() {
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
                    let existing = match Audio::find()
                        .filter(audio::Column::Uri.contains(&uri))
                        .one(&db)
                        .await
                    {
                        Ok(e) => e,
                        Err(e) => {
                            event!(Level::WARN, "Audio::find() failure: {}", e);
                            break;
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
                        let new_audio = Audio::insert(audio)
                            .exec(&db)
                            .await
                            .expect("failed to write audio details to database");

                        if let Some(tags) = tags {
                            for (name, values) in tags.iter_generic() {
                                event!(Level::DEBUG, "tag {}: {:?}", name, values);
                                for value in values {
                                    if let Some(s) = tags::get_tag_value(name, value) {
                                        let tag = audio_tag::ActiveModel {
                                            audio_id: ActiveValue::Set(new_audio.last_insert_id),
                                            name: ActiveValue::Set(name.to_string()),
                                            value: ActiveValue::Set(s.to_string()),
                                            ..Default::default()
                                        };
                                        event!(Level::DEBUG, "Insert AudioTag: {:?}", tag);
                                        AudioTag::insert(tag)
                                            .exec(&db)
                                            .await
                                            .expect("failed to write tag to database");

                                        // @TODO: @REMOVEME, instead query MusicBrainz in a queue
                                        // and properly populate this information. -- this is just
                                        // for testing the database schema.
                                        if name == "artist" {
                                            let existing_artist = match Artist::find()
                                                .filter(artist::Column::Name.contains(&s))
                                                .one(&db)
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
                                            };

                                            let artist_id = if let Some(artist) = existing_artist {
                                                artist.artist_id
                                            } else {
                                                let query =
                                                    musicbrainz_rs::entity::artist::Artist::query_builder()
                                                        .name(&s)
                                                        .build();
                                                let query_result = match
                                                    musicbrainz_rs::entity::artist::Artist::search(
                                                        query,
                                                    )
                                                    .execute() {
                                                        Ok(q) => q,
                                                        Err(e) => {
                                                            event!(Level::WARN, "musicbrainz query failed: {}", e);
                                                            // @TODO: leave the item in the queue to process later.
                                                            continue
                                                        }
                                                    };

                                                let artist = if let Some(result) =
                                                    // For now assume the first matching artist.
                                                    query_result.entities.get(0)
                                                {
                                                    event!(
                                                        Level::INFO,
                                                        "MusicBrainz response: {:#?}",
                                                        result
                                                    );

                                                    let mut area_id = 0;

                                                    // If area is defined, track it in the database.
                                                    if let Some(area) = &result.area {
                                                        let existing_area = match ArtistArea::find()
                                                            .filter(
                                                                artist_area::Column::Name
                                                                    .contains(&area.name),
                                                            )
                                                            .one(&db)
                                                            .await
                                                        {
                                                            Ok(e) => e,
                                                            Err(e) => {
                                                                event!(
                                                                    Level::WARN,
                                                                    "ArtistArea::find() failure: {}",
                                                                    e
                                                                );
                                                                break;
                                                            }
                                                        };
                                                        area_id = if let Some(area) = existing_area
                                                        {
                                                            area.artist_area_id
                                                        } else {
                                                            let new_area =
                                                                artist_area::ActiveModel {
                                                                    // @TODO:
                                                                    area_type: ActiveValue::Set(
                                                                        "".to_string(),
                                                                    ),
                                                                    name: ActiveValue::Set(
                                                                        area.name.to_string(),
                                                                    ),
                                                                    sort_name: ActiveValue::Set(
                                                                        area.sort_name.to_string(),
                                                                    ),
                                                                    disambiguation:
                                                                        ActiveValue::Set(
                                                                            area.disambiguation
                                                                                .to_string(),
                                                                        ),
                                                                    ..Default::default()
                                                                };
                                                            event!(
                                                                Level::DEBUG,
                                                                "Insert ArtistArea: {:?}",
                                                                new_area
                                                            );
                                                            let new_artist_area = ArtistArea::insert(new_area)
                                                                .exec(&db)
                                                                .await
                                                                .expect("failed to write artist to database");
                                                            new_artist_area.last_insert_id
                                                        };
                                                    }

                                                    // Artist AreaId is optional.
                                                    let artist_area_id = if area_id > 0 {
                                                        Some(area_id)
                                                    } else {
                                                        None
                                                    };

                                                    // ArtistType is optional, convert to RiaArtistType to add
                                                    // SeaOrm mapping.
                                                    let artist_type: Option<RiaArtistType> =
                                                        result.artist_type.as_ref().map(|a| {
                                                            a.try_into().expect(
                                                                "ArtistType conversion can't fail",
                                                            )
                                                        });

                                                    // Gender is optional, convert to RiaGender to add
                                                    // SeaOrm mapping.
                                                    let gender: Option<RiaGender> =
                                                        result.gender.as_ref().map(|g| {
                                                            g.try_into().expect(
                                                                "Gender conversion can't fail",
                                                            )
                                                        });

                                                    artist::ActiveModel {
                                                        name: ActiveValue::Set(
                                                            result.name.to_string(),
                                                        ),
                                                        sort_name: ActiveValue::Set(
                                                            result.sort_name.to_string(),
                                                        ),
                                                        disambiguation_comment: ActiveValue::Set(
                                                            result.disambiguation.to_string(),
                                                        ),
                                                        artist_area_id: ActiveValue::Set(
                                                            artist_area_id,
                                                        ),
                                                        artist_type: ActiveValue::Set(artist_type),
                                                        gender: ActiveValue::Set(gender),
                                                        ..Default::default()
                                                    }
                                                } else {
                                                    event!(
                                                        Level::WARN,
                                                        "{} not found in MusicBrainz",
                                                        &s
                                                    );
                                                    artist::ActiveModel {
                                                        name: ActiveValue::Set(s.to_string()),
                                                        ..Default::default()
                                                    }
                                                };

                                                event!(Level::DEBUG, "Insert Artist: {:?}", artist);
                                                let new_artist = Artist::insert(artist)
                                                    .exec(&db)
                                                    .await
                                                    .expect("failed to write artist to database");
                                                new_artist.last_insert_id
                                            };

                                            let audio_artist = audio_artist::ActiveModel {
                                                audio_id: ActiveValue::Set(
                                                    new_audio.last_insert_id,
                                                ),
                                                artist_id: ActiveValue::Set(artist_id),
                                                ..Default::default()
                                            };

                                            event!(
                                                Level::DEBUG,
                                                "Insert AudioArtist: {:?}",
                                                audio_artist
                                            );
                                            AudioArtist::insert(audio_artist)
                                                .exec(&db)
                                                .await
                                                .expect("failed to write audio_artist to database");
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

    Ok(())
}
