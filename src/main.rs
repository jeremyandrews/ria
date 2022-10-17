use std::error::Error;
use std::str::FromStr;

use file_format::FileFormat;
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
                    println!(
                        "AUDIO ({}): {}",
                        format.media_type(),
                        entry.as_ref().unwrap().path().display()
                    );
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
        // Albums are collected together in directories.
        } else if metadata.is_dir() {
            // @TODO: Track directories for visualization, organization, and to assist in
            // auto-identifying albums.
        }
    }
}
