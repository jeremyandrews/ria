pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_audio_table;
mod m20220101_000002_create_audiotag_table;
mod m20220101_000003_create_image_table;
mod m20220101_000004_create_artistarea_table;
mod m20220101_000005_create_artist_table;
mod m20220101_000006_create_audioartist_table;
mod m20220101_000007_create_musicbrainzqueue_table;
mod m20220101_000008_create_directory_table;
mod m20220101_000009_create_artistdirectory_table;
mod m20220101_000010_create_audiodirectory_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_audio_table::Migration),
            Box::new(m20220101_000002_create_audiotag_table::Migration),
            Box::new(m20220101_000003_create_image_table::Migration),
            Box::new(m20220101_000004_create_artistarea_table::Migration),
            Box::new(m20220101_000005_create_artist_table::Migration),
            Box::new(m20220101_000006_create_audioartist_table::Migration),
            Box::new(m20220101_000007_create_musicbrainzqueue_table::Migration),
            Box::new(m20220101_000008_create_directory_table::Migration),
            Box::new(m20220101_000009_create_artistdirectory_table::Migration),
            Box::new(m20220101_000010_create_audiodirectory_table::Migration),
        ]
    }
}
