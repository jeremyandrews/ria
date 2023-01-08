// One or more Artsist can be associated with each directory.

use super::m20220101_000005_create_artist_table::Artist;
use super::m20220101_000008_create_directory_table::Directory;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the artist_directory table.
        manager
            .create_table(
                Table::create()
                    .table(ArtistDirectory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtistDirectory::ArtistDirectoryId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ArtistDirectory::Created)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistDirectory::Updated)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArtistDirectory::ArtistId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-artist-artistid")
                            .from(ArtistDirectory::Table, ArtistDirectory::ArtistId)
                            .to(Artist::Table, Artist::ArtistId),
                    )
                    .col(
                        ColumnDef::new(ArtistDirectory::DirectoryId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-directory-directoryid")
                            .from(ArtistDirectory::Table, ArtistDirectory::DirectoryId)
                            .to(Directory::Table, Directory::DirectoryId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ArtistDirectory::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum ArtistDirectory {
    Table,
    ArtistDirectoryId,
    Created,
    Updated,
    ArtistId,
    DirectoryId,
}
