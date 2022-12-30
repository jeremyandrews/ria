# Ria

An audio management system.


## Roadmap

 * CLI management and playing of local audio files
 * CLI search and recommendations
 * Expose functionality via API for web-based UI


## Assumptions

These are generally intended to be temporary assumptions, fixed/removed as development matures.

 * All music lives in `music/` within the `ria` directory (this can be a symbolic link)
 * Primary focus is on `flac` music files, secondary will be `mp3`
 * PostgreSQL database (intended to also support MySQL or SQLite, through SeaORM -- I'm also still experimenting with using an ORM versus raw queries)
    * in the Docker container the database is dropped and recreated each time it's started
 * Runs on Debian/Ubuntu Linux (see `docker-compose.yml` and `Dockerfile`)


## Installation Notes

Ria uses [`symphonia`](https://docs.rs/symphonia) to identify, validate, and play music files.

A good example of how to use Symphonia can be found in this example:
https://github.com/pdeljanov/Symphonia/blob/master/symphonia-play/src/main.rs

## Docker

A local development environment can be created using Docker and Docker Compose by checking out the ria codebase with git, and then from within `ria/` run the following command:
```bash
docker-compose up --build
```

It will take some time to download and install all dependencies. Once complete, you can connect into the `ria_ria` container as follows:
```bash
docker exec -it `docker ps | grep ria-ria | awk '{print $1}'` /bin/bash
```

The `ria/` codebase can be found in `/app`. For example:
```bash
% docker exec -it `docker ps | grep ria-ria | awk '{print $1}'` /bin/bash
root@8a6b1779e907:/app# cargo run --release
```

To copy files from the docker container back to the host, try something like:
```
docker cp `docker ps | grep ria_ria | awk '{print $1}'`:/app/src/entities/artist.rs .
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

To drop the database (deleting everything) and reload it with the latest schema:
```
DATABASE_URL="postgres://ria:password@database/ria" sea-orm-cli migrate fresh
```

(If not using the provided docker image, you may need to run `cargo install sea-orm-cli` first and resolve any dependency issues.)

Additional tables will be needed to track the following information:
 - artists
 - albums
 - playlists
 - statistics

            1 |        1 | artist            | Amanda Fish
            2 |        1 | title             | Free
            3 |        1 | album             | Free
            4 |        1 | datetime          | 2018
            5 |        1 | track-number      | 12
            6 |        1 | genre             | Blues
            7 |        1 | album-artist      | Amanda Fish
            8 |        1 | album-disc-number | 1
            9 |        1 | album-disc-count  | 1
           10 |        1 | track-count       | 12
           11 |        1 | audio-codec       | Free Lossless Audio Codec (FLAC)


To connect to the database through the CLI, from bash within the ria image:
```
psql --host database --username ria
```


## Identifying music

Some APIs to explore:

 - https://musicbrainz.org/
   - https://musicbrainz.org/doc/MusicBrainz_API free for non-commercial use
   - implementing rate limiting: https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
   - https://docs.rs/musicbrainz_rs/latest/musicbrainz_rs/

 - https://acoustid.org/
   - https://acoustid.org/webservice free for non-commercial use
   - "all you need to do is register your application"
      o https://acoustid.org/login?return_url=https%3A%2F%2Facoustid.org%2Fnew-application
   - how to extract fingerprints: https://acoustid.org/chromaprint
      o https://github.com/acoustid/chromaprint/blob/master/src/chromaprint.h
      o https://github.com/jameshurst/rust-chromaprint
      o https://docs.rs/chromaprint/latest/chromaprint/index.html
      o pure rust implementation: https://docs.rs/rusty-chromaprint/latest/rusty_chromaprint/
      o https://github.com/darksv/rusty-chromaprint

 - https://www.audiotag.info/apisection
   - upload 10 second sound samples and identify artist, song, etc
   - 3 hours of identification free per month (1,000 tracks)

 - https://developer.tmsapi.com/page
   - rumored to be the largest database online
   - seems to focus on movies and tv though, not music

 - https://rapidapi.com/yashmakan261/api/shazam8/
   - shazam API, 300 searches per month free
