use std::{fmt, str, str::FromStr};

use anyhow::anyhow;
use async_once_cell::OnceCell;
use clap::Parser;
use musicbrainz_rs::entity::artist::{ArtistType, Gender};
use regex::RegexSet;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

use crate::Config;

static DB: OnceCell<DatabaseConnection> = OnceCell::new();

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
pub(crate) enum DatabaseType {
    MySQL,
    PostgreSQL,
    SQLite,
}
impl FromStr for DatabaseType {
    type Err = anyhow::Error;

    #[instrument]
    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        event!(Level::TRACE, "from_str");

        let supported_databases = RegexSet::new([
            r"(?i)^(my|mysql|maria|mariadb)$",
            r"(?i)^(po|post|postgres|postgresql)$",
            r"(?i)^(sqlite|lite)$",
        ])
        .expect("failed to compile supported_databases RegexSet");
        let matches = supported_databases.matches(s);
        if matches.matched(0) {
            Ok(DatabaseType::MySQL)
        } else if matches.matched(1) {
            Ok(DatabaseType::PostgreSQL)
        } else if matches.matched(2) {
            Ok(DatabaseType::SQLite)
        } else {
            Err(anyhow!("unrecognized database type: {}", s))
        }
    }
}

impl fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DatabaseType::MySQL => write!(f, "MySQL"),
            DatabaseType::PostgreSQL => write!(f, "PostgreSQL"),
            DatabaseType::SQLite => write!(f, "SQLite"),
        }
    }
}

// @TODO: eventually this should auto-create the database schema.
#[instrument]
pub(crate) async fn connection(config: &Config) -> &'static DatabaseConnection {
    event!(Level::TRACE, "connection");

    let database_base = match config.database_type {
        DatabaseType::SQLite => "sqlite://",
        DatabaseType::MySQL => "mysql://",
        DatabaseType::PostgreSQL => "postgresq://",
    };

    let database_url = match config.database_type {
        DatabaseType::SQLite => {
            let default_db_name = "ria.db".to_string();
            let database_name = config.database_name.as_ref().unwrap_or(&default_db_name);
            format!("{}{}", database_base, database_name)
        }
        DatabaseType::MySQL | DatabaseType::PostgreSQL => {
            let default_db_name = "ria".to_string();
            let default_db_user = "ria".to_string();
            format!(
                "{}{}:{}@{}",
                database_base,
                config.database_user.as_ref().unwrap_or(&default_db_user),
                config
                    .database_password
                    .as_ref()
                    .expect("database password required"),
                config.database_name.as_ref().unwrap_or(&default_db_name),
            )
        }
    };

    event!(Level::WARN, "database url: {}", database_url);

    DB.get_or_init(async {
        match Database::connect(database_url).await {
            Ok(d) => d,
            Err(e) => {
                // @TODO: better error handling / perhaps retry.
                panic!("database error: {}", e);
            }
        }
    })
    .await
}

/// The recognized `artist.artist_type` options, as defined at
/// https://musicbrainz.org/doc/Artist. This enum is used in
/// `src/entities/artis.rs` and makes the data in the database
/// more human readable.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(64))")]
pub enum RiaArtistType {
    #[sea_orm(string_value = "person")]
    Person,
    #[sea_orm(string_value = "group")]
    Group,
    #[sea_orm(string_value = "orchestra")]
    Orchestra,
    #[sea_orm(string_value = "choir")]
    Choir,
    #[sea_orm(string_value = "character")]
    Character,
    #[sea_orm(string_value = "other")]
    Other,
}
impl From<&ArtistType> for RiaArtistType {
    #[instrument]
    fn from(artist_type: &ArtistType) -> Self {
        event!(Level::TRACE, "from");
        match *artist_type {
            ArtistType::Character => RiaArtistType::Character,
            ArtistType::Choir => RiaArtistType::Choir,
            ArtistType::Group => RiaArtistType::Group,
            ArtistType::Orchestra => RiaArtistType::Orchestra,
            ArtistType::Other => RiaArtistType::Other,
            ArtistType::Person => RiaArtistType::Person,
        }
    }
}

/// The recognized `artist.gender` options, as defined at
/// https://musicbrainz.org/doc/Artist. This enum is used in
/// `src/entities/artis.rs` and makes the data in the database
/// more human readable.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(64))")]
pub enum RiaGender {
    #[sea_orm(string_value = "male")]
    Male,
    #[sea_orm(string_value = "female")]
    Female,
    #[sea_orm(string_value = "other")]
    Other,
}
impl From<&Gender> for RiaGender {
    #[instrument]
    fn from(gender: &Gender) -> Self {
        event!(Level::TRACE, "from");
        match *gender {
            Gender::Male => RiaGender::Male,
            Gender::Female => RiaGender::Female,
            Gender::Other => RiaGender::Other,
        }
    }
}
