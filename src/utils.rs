use tracing::{event, instrument, Level};
use walkdir::DirEntry;

/// Skip files and directories that start with ".".
#[instrument]
pub(crate) fn is_hidden(entry: &DirEntry) -> bool {
    event!(Level::TRACE, "is_hidden");
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

// Dynamically build a user agent from package name and package version. Store
// in a OnceCell to allow static lifetime necessary for the MusicBrainz agent.
#[instrument]
pub(crate) fn build_user_agent() -> String {
    let user_agent = format!(
        "{}/{} (https://github.com/jeremyandrews/ria)",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    event!(Level::TRACE, "build_user_agent: {}", user_agent);
    user_agent
}
