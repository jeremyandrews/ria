use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the musicbrainz_queue table.
        manager
            .create_table(
                Table::create()
                    .table(MusicbrainzQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MusicbrainzQueue::MusicbrainzQueueId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MusicbrainzQueue::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(MusicbrainzQueue::ProcessingStartedAt).timestamp().null())
                    .col(ColumnDef::new(MusicbrainzQueue::Errors).string().null())
                    .col(ColumnDef::new(MusicbrainzQueue::Payload).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MusicbrainzQueue::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum MusicbrainzQueue {
    Table,
    MusicbrainzQueueId,
    CreatedAt,
    ProcessingStartedAt,
    Errors,
    Payload,
}
