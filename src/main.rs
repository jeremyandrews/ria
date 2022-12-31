mod database;
mod entities;
mod media;
mod musicbrainz;
mod utils;

use clap::Parser;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::MakeWriterExt;

static USER_AGENT: Lazy<String> = Lazy::new(utils::build_user_agent);

#[derive(Parser, Debug, Serialize, Deserialize)]
struct Config {
    /// Override the path to the music library
    #[arg(short, long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    library: Option<String>,
    /// Scan the music library
    #[arg(short, long)]
    scan: bool,
    /// List contents of music library
    #[arg(long)]
    list: bool,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Use CLI options if set.
    //let config: Config = Figment::from(Serialized::defaults(Config::parse()))
    let config: Config = Figment::from(Toml::file("ria.toml"))
        // Otherwise use environment variables if set.
        .merge(Env::prefixed("RIA_"))
        // Otherwise use the configuration file.
        .merge(Serialized::defaults(Config::parse()))
        .extract()
        .unwrap();

    if config.library.is_none() {
        println!("\nUsage: ria --library <LIBRARY>\n");
        std::process::exit(0);
    }

    // Log all events to a file. @TODO: add configurable rolling file support.
    let logfile = RollingFileAppender::new(Rotation::NEVER, "./", "ria.log");
    // Log `INFO` and above to stdout.
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_writer(stdout.and(logfile))
        .init();

    // Dynamically build a user agent from package name and version, set as MusicBrainz user agent.
    musicbrainz_rs::config::set_user_agent(&USER_AGENT);

    if config.scan {
        // Spawn thread for processing the queue.
        tokio::spawn(async move { musicbrainz::process_queue().await });

        // @TODO: make it possible to scan regularly, and whenever files are added/changed.
        let mut scanned_media = false;

        // Loop for scanning library.
        loop {
            event!(Level::TRACE, "top of main loop");

            // For now we scan for files only one time when starting.
            if !scanned_media {
                scanned_media = true;
                // Copy library path to send to scan_media_files thread.
                let library = config
                    .library
                    .as_ref()
                    .expect("library must exist")
                    .to_string();
                tokio::spawn(async move { media::scan_media_files(&library).await });
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    if config.list {
        media::list_media().await;
    }

    Ok(())
}
