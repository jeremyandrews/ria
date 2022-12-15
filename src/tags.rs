use gstreamer::tags::GenericTagIter;
use gstreamer::GstValueExt;
use tracing::{event, instrument, Level};

/// Convert audio file tag to String for specific tags that we care about.
#[instrument]
pub(crate) fn get_tag_value(t: &str, v: &glib::SendValue) -> Option<String> {
    event!(Level::TRACE, "get_tag_value");

    // This list was derived from scanning 100,000 audio files and looking at the contained
    // tags:
    // {
    //   "acoustid-id": 317,
    //   "album": 64799,
    //   "album-artist": 41890,
    //   "album-artist-sortname": 622,
    //   "album-disc-count": 23581,
    //   "album-disc-number": 29428,
    //   "album-sortname": 30,
    //   "application-name": 7861,
    //   "artist": 64853,
    //   "artist-sortname": 778,
    //   "audio-codec": 66659,
    //   "beats-per-minute": 815,
    //   "bitrate": 113,
    //   "chromaprint-fingerprint": 13,
    //   "comment": 19259,
    //   "composer": 9995,
    //   "contact": 115,
    //   "copyright": 10761,
    //   "datetime": 60746,
    //   "description": 4206,
    //   "discid": 721,
    //   "extended-comment": 27470,
    //   "genre": 55359,
    //   "geo-location-name": 125,
    //   "image": 30422,
    //   "isrc": 9984,
    //   "language-code": 873,
    //   "maximum-bitrate": 111,
    //   "minimum-bitrate": 111,
    //   "musicbrainz-albumartistid": 1304,
    //   "musicbrainz-albumid": 1354,
    //   "musicbrainz-artistid": 1318,
    //   "musicbrainz-discid": 668,
    //   "musicbrainz-releasegroupid": 622,
    //   "musicbrainz-releasetrackid": 512,
    //   "musicbrainz-trackid": 1380,
    //   "organization": 5319,
    //   "performer": 1512,
    //   "preview-image": 82,
    //   "replaygain-album-gain": 2970,
    //   "replaygain-album-peak": 2915,
    //   "replaygain-reference-level": 631,
    //   "replaygain-track-gain": 4986,
    //   "replaygain-track-peak": 4883,
    //   "title": 64485,
    //   "title-sortname": 12,
    //   "track-count": 35632,
    //   "track-number": 64069,
    //   "version": 191,
    // }
    // @TODO: extract images, explore other tags (such as comment).
    let tags_to_store = vec![
        "album",
        "album-artist",
        "album-disc-number",
        "album-disc-count",
        "artist",
        "audio-codec",
        "datetime",
        "genre",
        "title",
        "track-number",
        "track-count",
    ];
    if tags_to_store.contains(&t) {
        if let Ok(s) = v.get::<&str>() {
            Some(s.to_string())
        } else if let Ok(serialized) = v.serialize() {
            Some(serialized.into())
        } else {
            None
        }
    } else {
        None
    }
}

// Extract tag names from GenericTagIter.
pub(crate) fn get_tags(name: &str, values: GenericTagIter) -> Vec<String> {
    let mut tags = Vec::new();
    for value in values {
        if let Some(s) = get_tag_value(name, value) {
            tags.push(s.to_string());
        }
    }
    return tags;
}
