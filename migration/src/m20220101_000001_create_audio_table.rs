use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the Audio table.
        manager
            .create_table(
                Table::create()
                    .table(Audio::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Audio::Aid)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Audio::Uri).string().not_null().unique_key())
                    .col(ColumnDef::new(Audio::Path).string().not_null())
                    .col(ColumnDef::new(Audio::Name).string().not_null())
                    .col(ColumnDef::new(Audio::Extension).string().not_null())
                    .col(ColumnDef::new(Audio::Format).string().not_null())
                    .col(ColumnDef::new(Audio::Duration).integer().not_null())
                    .col(ColumnDef::new(Audio::Channels).integer().not_null())
                    .col(ColumnDef::new(Audio::Bits).integer().not_null())
                    .col(ColumnDef::new(Audio::Hertz).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Audio::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Audio {
    Table,
    Aid,
    Uri,
    Path,
    Name,
    Extension,
    Format,
    Duration,
    Channels,
    Bits,
    Hertz,
}
