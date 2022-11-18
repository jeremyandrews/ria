//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "audio_artist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub audio_artist_id: i32,
    pub audio_id: i32,
    pub artist_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::artist::Entity",
        from = "Column::ArtistId",
        to = "super::artist::Column::ArtistId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Artist,
    #[sea_orm(
        belongs_to = "super::audio::Entity",
        from = "Column::AudioId",
        to = "super::audio::Column::AudioId",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Audio,
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl Related<super::audio::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Audio.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}