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


## Docker

A local development environment can be created using Docker and Docker Compose by checking out the ria codebase with git, and then from within `ria/` run the following command:
```bash
docker-compose up --build
```

It will take some time to download and install all dependencies. Once complete, you can connect into the `ria_ria` container as follows:
```bash
docker exec -it `docker ps | grep ria_ria | awk '{print $1}'` /bin/bash
```

The `ria/` codebase can be found in `/root/ria`. For example:
```bash
% docker exec -it `docker ps | grep ria_ria | awk '{print $1}'` /bin/bash
root@8a6b1779e907:/# cd /root/ria/
root@8a6b1779e907:~/ria# cargo run --release
```


## Schema

The following tables track the files Ria is aware of in the filesystem.

file
 - fid
 - path
 - name
 - extension
 - type

audio
 - aid
 - type
 - channels
 - bits
 - hertz

image
 - iid
 - type

text
 - tid
 - type

Additional tables track:
 - artists
 - albums
 - playlists
 - statistics
