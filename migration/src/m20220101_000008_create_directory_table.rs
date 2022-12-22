// Typically directories are 1:1 mapped to releases, recordings, or albums.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the directory table.
        manager
            .create_table(
                Table::create()
                    .table(Directory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Directory::DirectoryId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Directory::Created).timestamp().not_null())
                    .col(ColumnDef::new(Directory::Updated).timestamp().not_null())
                    .col(ColumnDef::new(Directory::Path).string().not_null())
                    .col(ColumnDef::new(Directory::Name).string().not_null())
                    // @TODO: Link Audio to Directory
                    // @TODO: Group CD1/CD2 subdirectories.
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Directory::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Directory {
    Table,
    DirectoryId,
    Created,
    Updated,
    Path,
    Name,
}
