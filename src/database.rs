use async_once_cell::OnceCell;
use sea_orm::*;
use tracing::{event, instrument, Level};

use musicbrainz_rs::entity::artist::{ArtistType, Gender};

static DB: OnceCell<DatabaseConnection> = OnceCell::new();

// @TODO: Move into env.
const DATABASE_URL: &str = "postgres://ria:password@database";
const DB_NAME: &str = "ria";

// @TODO: eventually this should auto-create the database schema.
#[instrument]
pub(crate) async fn connection() -> &'static DatabaseConnection {
    event!(Level::TRACE, "connection");

    DB.get_or_init(async {
        match Database::connect(format!("{}/{}", DATABASE_URL, DB_NAME)).await {
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
