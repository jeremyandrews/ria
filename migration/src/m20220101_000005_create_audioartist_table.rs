use sea_orm_migration::prelude::*;

use super::m20220101_000001_create_audio_table::Audio;
use super::m20220101_000004_create_artist_table::Artist;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the Audioartist table.
        manager
            .create_table(
                Table::create()
                    .table(Audioartist::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Audioartist::Aaid)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Audioartist::AudioAid).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                        .name("fk-audio-aid")
                        .from(Audioartist::Table, Audioartist::AudioAid)
                        .to(Audio::Table, Audio::Aid),
                    )
                    .col(ColumnDef::new(Audioartist::ArtistAid).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                        .name("fk-artist-aid")
                        .from(Audioartist::Table, Audioartist::ArtistAid)
                        .to(Artist::Table, Artist::Aid),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Audioartist::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Audioartist {
    Table,
    Aaid,
    AudioAid,
    ArtistAid,
}
