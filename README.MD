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

The database schema is controlled in SeaORM, and was built through the numbers files in `migrateion/src`. The tables are created from those files with the following command:
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
