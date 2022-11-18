use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the ArtistArea table.
        manager
            .create_table(
                Table::create()
                    .table(ArtistArea::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArtistArea::ArtistAreaId)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ArtistArea::AreaType).string().not_null())
                    .col(ColumnDef::new(ArtistArea::Name).string().not_null())
                    .col(ColumnDef::new(ArtistArea::SortName).string().not_null())
                    .col(ColumnDef::new(ArtistArea::Disambiguation).string().not_null())
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ArtistArea::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum ArtistArea {
    Table,
    ArtistAreaId,
    AreaType,
    Name,
    SortName,
    Disambiguation,
}
