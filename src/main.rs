mod database;
mod entities;
mod media;
mod musicbrainz;
mod player;
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

use crate::database::DatabaseType;

static USER_AGENT: Lazy<String> = Lazy::new(utils::build_user_agent);

#[derive(Clone, Debug, Parser, Serialize, Deserialize)]
struct Config {
    /// Set path to the music library
    #[arg(short, long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    library: Option<String>,
    /// Scan the music library
    #[arg(short, long)]
    scan: bool,
    /// Print contents of music library
    #[arg(long)]
    print: bool,
    /// Play selected music from library
    #[arg(short, long)]
    play: bool,

    /// Filter by artist
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    artist: Option<String>,
    /// Filter by track
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    track: Option<String>,
    /// Filter by directory
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    directory: Option<String>,

    /// Specify database type
    #[arg(short, long, default_value_t = DatabaseType::SQLite)]
    database_type: DatabaseType,
    /// Specify database name
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    database_name: Option<String>,
    /// Specify database user name
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    database_user: Option<String>,
    /// Specify database password
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    database_password: Option<String>,
    // @TODO: Figure out how to invoke rand()
    // /// Randomize music library listing
    //#[arg(long)]
    //random: bool,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Start with toml configuration file.
    let config: Config = Figment::from(Toml::file("ria.toml"))
        // Override with anything set in environment variables.
        .merge(Env::prefixed("RIA_"))
        // Override with anything set via flags.
        .merge(Serialized::defaults(Config::parse()))
        .extract()
        .unwrap();

    // Library must be configurex (typically in `ria.toml` or RIA_LIBRARY.)
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
        let process_config = config.clone();
        // Spawn thread for processing the queue.
        tokio::spawn(async move { musicbrainz::process_queue(&process_config).await });

        // @TODO: make it possible to scan regularly, and whenever files are added/changed.
        let mut scanned_media = false;

        // Loop for scanning library.
        loop {
            event!(Level::TRACE, "top of main loop");

            // For now we scan for files only one time when starting.
            if !scanned_media {
                scanned_media = true;
                // Copy config structure to send to scan_media_files thread.
                let scan_config = config.clone();
                tokio::spawn(async move { media::scan_media_files(&scan_config).await });
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    if config.print {
        media::print_media(&config).await;
    }

    if config.play {
        let audio_files = media::get_media(&config).await;
        for audio_file in audio_files {
            let audio_file_name = format!("{}/{}", audio_file.audio_path, audio_file.audio_name);
            event!(
                Level::WARN,
                "playing audio file {}: {}",
                audio_file.audio_id,
                audio_file_name
            );
            player::play_audio(&audio_file_name);
        }
    }

    Ok(())
}
