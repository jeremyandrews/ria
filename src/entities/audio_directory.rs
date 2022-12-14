//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "audio_directory")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub audio_directory_id: i32,
    pub created: DateTime,
    pub updated: DateTime,
    pub directory_id: i32,
    pub audio_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::audio::Entity",
        from = "Column::AudioId",
        to = "super::audio::Column::AudioId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Audio,
    #[sea_orm(
        belongs_to = "super::directory::Entity",
        from = "Column::DirectoryId",
        to = "super::directory::Column::DirectoryId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Directory,
}

impl Related<super::audio::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Audio.def()
    }
}

impl Related<super::directory::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Directory.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
