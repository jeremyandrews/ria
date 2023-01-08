// One or more track can be associated with each directory.

use super::m20220101_000001_create_audio_table::Audio;
use super::m20220101_000008_create_directory_table::Directory;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the audio_directory table.
        manager
            .create_table(
                Table::create()
                    .table(AudioDirectory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AudioDirectory::AudioDirectoryId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AudioDirectory::Created)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AudioDirectory::Updated)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AudioDirectory::DirectoryId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-directory-directoryid")
                            .from(AudioDirectory::Table, AudioDirectory::DirectoryId)
                            .to(Directory::Table, Directory::DirectoryId),
                    )
                    .col(ColumnDef::new(AudioDirectory::AudioId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-audio-audaioid")
                            .from(AudioDirectory::Table, AudioDirectory::AudioId)
                            .to(Audio::Table, Audio::AudioId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AudioDirectory::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum AudioDirectory {
    Table,
    AudioDirectoryId,
    Created,
    Updated,
    DirectoryId,
    AudioId,
}
