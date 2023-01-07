# Ria

An audio management system.


## Roadmap

 * improve artist, album, and track detection/identification
    o match directories to musicbrainz releases
    o get and present information about aritsts and releases
 * improve CLI management and playing of local audio files
 * explore playing audio files remotely (airplay?)
    o test https://github.com/arilence/airplay-rs
    o unfortunately: https://github.com/arilence/airplay-rs/issues/1
 * integrate comprehensive search
    o test https://docs.rs/tantivy/latest/tantivy/
 * automate creation of database
 * implement recommendation system, "if you like this, you may also like ..."
 * expose all features through an API for a web-based UI


## Getting Started

Ria can be configured (in order of precedence) by setting CLI flags, by setting environment variables, and via the `ria.toml` configuration file. The default configuration assumes all audio files live in a `./music` directory. It defaults to using SQLite in a file named `./ria.db`. For large audio collections it's strongly recommended to use `PostgreSQL` or `MySQL` instead.

1) Create the SQLite database file with the following commaned: `touch ria.db`
2) Scan all audio files: `cargo run --release -- --scan`
3) List all detected audio files: `cargo run --release -- --print`
4) Filter selected audio files with `--artist`, `--directory` and `--track` options
5) Play filtered audio files by adding the `--play` flag


## Database

For ease of development, Ria defaults to using SQLite. All database connections are managed through SeaORM making it possible to also use PostgreSQL and MySQL by simply configuring `database_type` in `ria.toml`, `RIA_DATABASE_TYPE`, or the `--database-type` command line configuration option. If using SQLite you just need to specify a `database_name`. If using PostgreSQL or MySQL you also need to specify a `database_user` and a `database_password`.

### Creation

It is on the roadmap for Ria to auto-create the database when necessary, but this has not yet been implemented.

[SeaORM](https://docs.rs/sea-orm/) CLI is required to manually run the database migrations:
```
cargo install sea-orm-cli
```

#### SQLite

There are two steps to create the Ria SQLite database:

1) Create an empty database file, for example: `touch ria.db`
2) Run the database migration: `DATABASE_URL="sqlite://ria.db" sea-orm-cli migrate fresh`

#### PostgreSQL or MySQL

To create the Ria PostgreSQL database, first create a database and a database user with read/write access. Then, run the database migration. For example, if the database_name is `ria`, the database_user is `ria` and the database_password `password`, for PostgreSQL run the command as follows:
```
DATABASE_URL="postgres://ria:password@database/ria" sea-orm-cli migrate fresh
```

For MySQL run the following command:
```
DATABASE_URL="mysql://ria:password@database/ria" sea-orm-cli migrate fresh
```


## Implementation Notes

Ria depends on a number of packages, including:
 - [`symphonia`](https://docs.rs/symphonia) is used to identify, validate, and decode audio files;
 - [`rodio`](https://docs.rs/rodio) is used to locally play music files;
 - [`sea-orm`](https://docs.rs/sea-orm/) is used to communicat with the database, supporting SQLite, PostgreSQL and MySQL.


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

The docker image uses the PostgreSQL database. To connect to the database through the CLI, from bash within the ria docker image:
```
psql --host database --username ria
```


## Schema

The underlying database schema is controlled by SeaORM, and was built through the numbered files in `migration/src`. The following command drops the database (deleting everything) and reloads it with the latest schema:
```
sea-orm-cli migrate fresh
```

Individual entity files are auto-generated in `src/entities/` by running the following SeaORM command:
```
sea-orm-cli generate entity -u postgres://ria:password@database/ria -o src/entities
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
