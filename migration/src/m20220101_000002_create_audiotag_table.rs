use sea_orm_migration::prelude::*;

use super::m20220101_000001_create_audio_table::Audio;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the Audiotag table.
        manager
            .create_table(
                Table::create()
                    .table(Audiotag::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Audiotag::Tid)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Audiotag::Aid).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                        .name("fk-audio-aid")
                        .from(Audiotag::Table, Audiotag::Aid)
                        .to(Audio::Table, Audio::Aid),
                    )
                    .col(ColumnDef::new(Audiotag::Name).string().not_null())
                    .col(ColumnDef::new(Audiotag::Value).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Audiotag::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Audiotag {
    Table,
    Tid,
    Aid,
    Name,
    Value,
}
