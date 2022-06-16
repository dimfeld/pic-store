use sea_orm::entity::prelude::TimeDateTimeWithTimeZone;
use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "upload_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub project_id: Uuid,
    pub name: String,

    /// Where to store the input images, since they may not want to be in the same place as the
    /// output.
    pub base_storage_location_ud: Uuid,
    /// Where to store the converted output images.
    pub output_storage_location_id: Uuid,
    pub conversion_profile_id: Uuid,

    pub updated: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
    #[sea_orm(
        belongs_to = "super::storage_location::Entity",
        from = "Column::StorageLocationId",
        to = "super::storage_location::Column::Id"
    )]
    StorageLocation,
    #[sea_orm(
        belongs_to = "super::conversion_profile::Entity",
        from = "Column::ConversionProfileId",
        to = "super::conversion_profile::Column::Id"
    )]
    ConversionProfile,
}

impl Related<super::conversion_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConversionProfile.def()
    }
}
impl Related<super::storage_location::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StorageLocation.def()
    }
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
