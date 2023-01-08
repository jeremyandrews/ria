use sea_orm_migration::prelude::*;

use super::m20220101_000001_create_audio_table::Audio;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the AudioTag table.
        manager
            .create_table(
                Table::create()
                    .table(AudioTag::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AudioTag::AudioTagId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AudioTag::AudioId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-audio-audioid")
                            .from(AudioTag::Table, AudioTag::AudioId)
                            .to(Audio::Table, Audio::AudioId),
                    )
                    .col(ColumnDef::new(AudioTag::Name).string().not_null())
                    .col(ColumnDef::new(AudioTag::Value).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AudioTag::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum AudioTag {
    Table,
    AudioTagId,
    AudioId,
    Name,
    Value,
}
