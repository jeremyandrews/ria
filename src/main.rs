use std::error::Error;
use std::str::FromStr;

use file_format::FileFormat;
use gstreamer_pbutils::DiscovererAudioInfo;
use gstreamer_pbutils::{prelude::*, DiscovererContainerInfo};
use percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use walkdir::{DirEntry, WalkDir};

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

    fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
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

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn main() {
    // Initialize GStreamer.
    // @TODO: Error handling.
    gstreamer::init().unwrap();

    // @TODO: Make directories configurable.
    let walker = WalkDir::new("music").follow_links(true).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let metadata = entry.as_ref().unwrap().metadata().unwrap();

        // Files may be tracks, images, playlists, and more.
        if metadata.is_file() {
            let format = FileFormat::from_file(entry.as_ref().unwrap().path()).unwrap();
            let media_type = MediaType::from_str(format.media_type()).unwrap();
            match media_type {
                MediaType::Image => {
                    // @TODO: Associate the image with the album or artist depending on where it is.
                    println!(
                        "IMAGE ({}): {}",
                        format.media_type(),
                        entry.as_ref().unwrap().path().display()
                    );
                }
                MediaType::_Text => {
                    // @TODO: How to properly detect text?
                }
                MediaType::Audio => {
                    // Build an absolute URI as required by GStreamer.
                    let uri = format!(
                        "file:///{}/{}",
                        std::env::current_dir().unwrap().display(),
                        entry.as_ref().unwrap().path().display()
                    );

                    println!("Name: {}", entry.as_ref().unwrap().path().display());

                    let timeout: gstreamer::ClockTime = gstreamer::ClockTime::from_seconds(15);
                    let discoverer = gstreamer_pbutils::Discoverer::new(timeout).unwrap();
                    let info = discoverer
                        .discover_uri(
                            &utf8_percent_encode(&uri, DEFAULT_ENCODE_SET).collect::<String>(),
                        )
                        .unwrap();

                    println!("Duration: {}", info.duration().unwrap());

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
                        println!("Format: {}", caps_str);

                        if let Some(container_info) =
                            stream_info.downcast_ref::<DiscovererContainerInfo>()
                        {
                            println!(
                                "@TODO @@@@@@@@@@: Handle containers... {:#?}",
                                container_info
                            );
                        } else if let Some(container_audio) =
                            stream_info.downcast_ref::<DiscovererAudioInfo>()
                        {
                            println!(
                                "{} channel: {}-bit {} hz",
                                container_audio.channels(),
                                container_audio.depth(),
                                container_audio.sample_rate()
                            );
                            println!(
                                "{} bitrate, {} max bitrate, {:?} language",
                                container_audio.bitrate(),
                                container_audio.max_bitrate(),
                                container_audio.language()
                            );
                        } else {
                            println!("@TODO @@@@@@@@@@: Handle non-audio streams");
                        }
                    }
                }
                MediaType::Unknown => {
                    // @TODO: Deal with audio files that we didn't properly detect.
                    // @TODO: Perhaps detect text files in a second pass here, on the file extension?
                    println!(
                        "UNKNOWN ({}): {}",
                        format.media_type(),
                        entry.as_ref().unwrap().path().display()
                    );
                }
            }
            println!("------------------")
        // Albums are collected together in directories.
        } else if metadata.is_dir() {
            // @TODO: Track directories for visualization, organization, and to assist in
            // auto-identifying albums.
        }
    }
}
