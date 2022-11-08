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

And entity files are then auto-generated in `src/entities/` with the following command:
```
sea-orm-cli generate entity -u postgres://ria:password@database/ria -o src/entities
```

(If not using the provided docker image, you may need to run `cargo install sea-orm-cli` first and resolve any dependency issues.)

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

Processing 10,000 flac audio files contained the following tags:
    All tags identified: {
        "acoustid-id": 60,
        "album": 5776,
        "album-artist": 3525,
        "album-artist-sortname": 90,
        "album-disc-count": 1425,
        "album-disc-number": 2042,
        "application-name": 986,
        "artist": 5795,
        "artist-sortname": 112,
        "audio-codec": 6059,
        "beats-per-minute": 132,
        "bitrate": 15,
        "comment": 1789,
        "composer": 494,
        "contact": 11,
        "copyright": 615,
        "datetime": 5524,
        "description": 393,
        "discid": 283,
        "extended-comment": 2373,
        "genre": 4927,
        "geo-location-name": 33,
        "image": 2836,
        "isrc": 587,
        "language-code": 111,
        "maximum-bitrate": 15,
        "minimum-bitrate": 15,
        "musicbrainz-albumartistid": 139,
        "musicbrainz-albumid": 139,
        "musicbrainz-artistid": 139,
        "musicbrainz-discid": 61,
        "musicbrainz-releasegroupid": 78,
        "musicbrainz-releasetrackid": 78,
        "musicbrainz-trackid": 139,
        "organization": 654,
        "performer": 48,
        "replaygain-album-gain": 377,
        "replaygain-album-peak": 377,
        "replaygain-reference-level": 127,
        "replaygain-track-gain": 579,
        "replaygain-track-peak": 579,
        "title": 5775,
        "track-count": 2739,
        "track-number": 5733,
        "version": 1,
    }

Processing 100,000 flac files (including those above) contained the following tags:
All tags identified: {
    "acoustid-id": 317,
    "album": 64799,
    "album-artist": 41890,
    "album-artist-sortname": 622,
    "album-disc-count": 23581,
    "album-disc-number": 29428,
    "album-sortname": 30,
    "application-name": 7861,
    "artist": 64853,
    "artist-sortname": 778,
    "audio-codec": 66659,
    "beats-per-minute": 815,
    "bitrate": 113,
    "chromaprint-fingerprint": 13,
    "comment": 19259,
    "composer": 9995,
    "contact": 115,
    "copyright": 10761,
    "datetime": 60746,
    "description": 4206,
    "discid": 721,
    "extended-comment": 27470,
    "genre": 55359,
    "geo-location-name": 125,
    "image": 30422,
    "isrc": 9984,
    "language-code": 873,
    "maximum-bitrate": 111,
    "minimum-bitrate": 111,
    "musicbrainz-albumartistid": 1304,
    "musicbrainz-albumid": 1354,
    "musicbrainz-artistid": 1318,
    "musicbrainz-discid": 668,
    "musicbrainz-releasegroupid": 622,
    "musicbrainz-releasetrackid": 512,
    "musicbrainz-trackid": 1380,
    "organization": 5319,
    "performer": 1512,
    "preview-image": 82,
    "replaygain-album-gain": 2970,
    "replaygain-album-peak": 2915,
    "replaygain-reference-level": 631,
    "replaygain-track-gain": 4986,
    "replaygain-track-peak": 4883,
    "title": 64485,
    "title-sortname": 12,
    "track-count": 35632,
    "track-number": 64069,
    "version": 191,
}

So within this sample, start by focusing on the following tags:
    "audio-codec": 66659,
    "track-number": 64069,
    "datetime": 60746,
    "artist": 64853,
    "album": 64799,
    "title": 64485,
    "genre": 55359,
    "album-artist": 41890,
    "track-count": 35632,
    "album-disc-number": 29428,
    "album-disc-count": 23581,