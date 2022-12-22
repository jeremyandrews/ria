//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "musicbrainz_queue")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub musicbrainz_queue_id: i32,
    pub created_at: DateTime,
    pub processing_started_at: Option<DateTime>,
    pub errors: Option<String>,
    pub payload: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}