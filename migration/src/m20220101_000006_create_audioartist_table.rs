use sea_orm_migration::prelude::*;

use super::m20220101_000001_create_audio_table::Audio;
use super::m20220101_000005_create_artist_table::Artist;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the AudioArtist table.
        manager
            .create_table(
                Table::create()
                    .table(AudioArtist::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AudioArtist::AudioArtistId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AudioArtist::AudioId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                        .name("fk-audio-audioid")
                        .from(AudioArtist::Table, AudioArtist::AudioId)
                        .to(Audio::Table, Audio::AudioId),
                    )
                    .col(ColumnDef::new(AudioArtist::ArtistId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                        .name("fk-artist-artistid")
                        .from(AudioArtist::Table, AudioArtist::ArtistId)
                        .to(Artist::Table, Artist::ArtistId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AudioArtist::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum AudioArtist {
    Table,
    AudioArtistId,
    AudioId,
    ArtistId,
}
