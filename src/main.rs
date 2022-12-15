mod database;
mod entities;
mod media;
mod musicbrainz;
mod tags;
mod utils;

use once_cell::sync::Lazy;
use tracing::{event, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;

static USER_AGENT: Lazy<String> = Lazy::new(|| utils::build_user_agent());

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Log all events to a file. @TODO: add configurable rolling file support.
    let logfile = RollingFileAppender::new(Rotation::NEVER, "./", "ria.log");
    // Log `INFO` and above to stdout.
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_writer(stdout.and(logfile))
        .init();

    // Initialize GStreamer.
    gstreamer::init().expect("failed to initialize gstreamer");

    // Dynamically build a user agent from package name and version, set as MusicBrainz user agent.
    musicbrainz_rs::config::set_user_agent(&*USER_AGENT);

    // Spawn thread for processing the queue.
    tokio::spawn(async move { musicbrainz::process_queue().await });

    // @TODO: make it possible to scan regularly, and whenever files are added/changed.
    let mut scanned_media = false;

    // Main loop.
    loop {
        event!(Level::TRACE, "top of main loop");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // For now we scan for files only one time when starting.
        if !scanned_media {
            scanned_media = true;
            // @TODO: Make directories configurable. Currently hardcoded for `./music/`.
            tokio::spawn(async move { media::scan_media_files("./music/").await });
        }
    }

    // @TODO: Provide a way to exit ria.
    //Ok(())
}
