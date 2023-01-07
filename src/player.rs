use std::io::BufReader;
use tracing::{event, instrument, Level};

#[instrument]
pub(crate) fn play_audio(path_str: &str) {
    event!(Level::TRACE, "play_audio {}", path_str);

    // Create a rodio sink representing the audio track.
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();

    // Load file into the sink.
    let file = std::fs::File::open(path_str).unwrap();
    sink.append(match rodio::Decoder::new(BufReader::new(file)) {
        Ok(d) => d,
        Err(e) => {
            event!(Level::ERROR, "play_audio {} error: {}", path_str, e);
            return;
        }
    });

    // Block until the sink has finished playing.
    sink.sleep_until_end();
}
