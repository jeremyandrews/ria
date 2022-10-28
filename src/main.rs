mod database;
mod entities;

use std::error::Error;
use std::path::Path;
use std::str::FromStr;

use file_format::FileFormat;
use gstreamer_pbutils::DiscovererAudioInfo;
use gstreamer_pbutils::{prelude::*, DiscovererContainerInfo};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sea_orm::*;
use tracing::{event, instrument, Level};
use tracing_subscriber::FmtSubscriber;
use walkdir::{DirEntry, WalkDir};

use entities::{prelude::*, *};

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
    type Err = Box<dyn Error>;

    #[instrument]
    fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
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
    // Display INFO and higher level logs.
    // @TODO: Make this configurable.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Initialize the database.
    // @TODO: Error handling.
    let db = database::connection()
        .await
        .expect("failed to connect to database");

    // Initialize GStreamer.
    gstreamer::init().expect("failed to initialize gstreamer");

    // Percent-encode all characters except alpha-numerics and "/" to build proper
    // paths.
    // @TODO: remove characters necessary to navigate Windows paths.
    const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/');

    // @TODO: Make directories configurable.
    let walker = WalkDir::new("music").follow_links(true).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
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
                        "file:///{}",
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
                    let mut audio = audio::ActiveModel {
                        // @TODO: replace unwrap() with proper error handling.
                        path: ActiveValue::Set(path.parent().unwrap().display().to_string()),
                        // @TODO: replace unwrap() with proper error handling.
                        name: ActiveValue::Set(
                            path.file_name().unwrap().to_str().unwrap().to_string(),
                        ),
                        // @TODO: replace unwrap() with proper error handling.
                        extension: ActiveValue::Set(
                            path.extension().unwrap().to_str().unwrap().to_string(),
                        ),
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
                    audio.duration = sea_orm::ActiveValue::Set(
                        info.duration().unwrap().mseconds().try_into().unwrap(),
                    );

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
                            /*
                            println!(
                                "{} bitrate, {} max bitrate, {:?} language",
                                container_audio.bitrate(),
                                container_audio.max_bitrate(),
                                container_audio.language()
                            );
                             */
                        } else {
                            event!(Level::WARN, "@TODO @@@@@@@@@@: Handle non-audio streams");
                        }
                    }
                    // @TODO: Error handling.
                    event!(Level::INFO, "Insert Audio File: {:?}", audio);
                    Audio::insert(audio)
                        .exec(&db)
                        .await
                        .expect("failed to write to database");
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
    }

    Ok(())
}
