use sea_orm::*;
use tracing::{event, instrument, Level};

// @TODO: Move into env.
const DATABASE_URL: &str = "postgres://ria:password@database";
const DB_NAME: &str = "ria";

// @TODO: eventually this should auto-create the database schema.
#[instrument]
pub(crate) async fn connection() -> Result<DatabaseConnection, DbErr> {
    event!(Level::TRACE, "connection");
    Database::connect(format!("{}/{}", DATABASE_URL, DB_NAME)).await
}

/// The recognized `artist.artist_type` options, as defined at
/// https://musicbrainz.org/doc/Artist. This enum is used in
/// `src/entities/artis.rs`.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(64))")]
pub enum ArtistType {
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

/// The recognized `artist.gender` options, as defined at
/// https://musicbrainz.org/doc/Artist. This enum is used in
/// `src/entities/artis.rs`.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(64))")]
pub enum Gender {
    #[sea_orm(string_value = "male")]
    Male,
    #[sea_orm(string_value = "female")]
    Female,
    #[sea_orm(string_value = "nonbinary")]
    Nonbinary,
}
