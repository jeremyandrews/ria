use sea_orm::*;
use tracing::{event, instrument, Level};

// @TODO: Move into env.
const DATABASE_URL: &str = "postgres://ria:password@database";
const DB_NAME: &str = "ria";

// @TODO: eventually this should auto-create the database schema.
#[instrument]
pub(crate) async fn connection() -> Result<DatabaseConnection, DbErr> {
    event!(Level::TRACE, "connection");
    Ok(Database::connect(format!("{}/{}", DATABASE_URL, DB_NAME)).await?)
}
