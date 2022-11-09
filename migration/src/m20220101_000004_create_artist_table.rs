use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// The Artist table is largely derived from the MusicBrainz database:
// https://musicbrainz.org/doc/Artist

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the Artist table.
        manager
            .create_table(
                Table::create()
                    .table(Artist::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Artist::Aid)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Artist::Name).string().not_null())
                    .col(ColumnDef::new(Artist::SortName).string().null())
                    .col(ColumnDef::new(Artist::ArtistType).string().null())
                    .col(ColumnDef::new(Artist::Gender).string().null())
                    .col(ColumnDef::new(Artist::Area).string().null())
                    // @TODO: Alias (multiple, external table)
                    // @TODO: MBID (multiple, external table)
                    .col(ColumnDef::new(Artist::DisambiguationComment).string().null())
                    // @TODO: Annotation
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Artist::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Artist {
    Table,
    Aid,
    Name,
    SortName,
    ArtistType,
    Gender,
    Area,
    DisambiguationComment,
}
