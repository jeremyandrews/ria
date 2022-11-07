# Ria

An audio management system.


## Roadmap

 * CLI management and playing of local audio files
 * CLI search and recommendations
 * Expose functionality via API for web-based UI


## Assumptions

These are generally intended to be temporary assumptions, fixed/removed as development matures.

 * All music lives in `music/` within the `ria` directory (this can be a symbolic link)
 * Primary focus is on `flac` music files
 * PostgreSQL database (intended to also support MySQL or SQLite, through SeaORM -- I'm also still experimenting with using an ORM versus raw queries)
 * Runs on Debian/Ubuntu Linux (see `docker-compose.yml` and `container/Dockerfile`)


## Installation Notes

Ria uses `gstreamer` to identify, validate, and play music files.

Follow the official GStreamer documentation to install this dependency, and all desired plugins:
  https://gstreamer.freedesktop.org/documentation/installing/

It is likely helpful to review the instructions for the `gstreamer-rs` bindings used by Ria:
  https://gstreamer.pages.freedesktop.org/gstreamer-rs/stable/latest/docs/gstreamer_audio/

Initial gstreamer-rs implementation for identifying music files was derived from [basic-tutorial-9.rs](https://github.com/sdroege/gstreamer-rs/blob/main/tutorials/src/bin/basic-tutorial-9.rs), which is based on [Basic tutorial 9: Media information gathering](https://gstreamer.freedesktop.org/documentation/tutorials/basic/media-information-gathering.html?gi-language=c).


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

The database schema is controlled in SeaORM, and was built through the numbered files in `migration/src`. The tables are created from those files with the following command:
```
DATABASE_URL="postgres://ria:password@database/ria" sea-orm-cli migrate refresh
```

Additional tables will be needed to track the following information:
 - artists
 - albums
 - playlists
 - statistics

To connect to the database through the CLI, from bash within the ria image:
```
psql --host database --username ria
```


## Identifying music

Some APIs to explore:

 - https://musicbrainz.org/
   - https://musicbrainz.org/doc/MusicBrainz_API non commercial use is free
   - implementing rate limiting: https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
   - https://docs.rs/musicbrainz_rs/latest/musicbrainz_rs/

 - https://www.audiotag.info/apisection
   - upload 10 second sound samples and identify artist, song, etc
   - 3 hours of identification free per month (1,000 tracks)

 - https://developer.tmsapi.com/page
   - rumored to be the largest database online
   - seems to focus on movies and tv though, not music

 - https://rapidapi.com/yashmakan261/api/shazam8/
   - shazam API, 300 searches per month free
