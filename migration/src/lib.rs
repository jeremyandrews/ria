pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_audio_table;
mod m20220101_000002_create_audiotag_table;
mod m20220101_000003_create_image_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_audio_table::Migration),
            Box::new(m20220101_000002_create_audiotag_table::Migration),
            Box::new(m20220101_000003_create_image_table::Migration),
        ]
    }
}
