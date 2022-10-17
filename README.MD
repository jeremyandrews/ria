# Ria

An audio management system.


## Roadmap

 * CLI management and playing of local audio files
 * CLI search and recommendations
 * Expose functionality via API for web-based UI


## Installation Notes

Ria uses `gstreamer` to identify, validate, and play music files.

Follow the official GStreamer documentation to install this dependency, and all desired plugins:
  https://gstreamer.freedesktop.org/documentation/installing/

It is likely helpful to review the instructions for the `gstreamer-rs` bindings used by Ria:
  https://gstreamer.pages.freedesktop.org/gstreamer-rs/stable/latest/docs/gstreamer_audio/
