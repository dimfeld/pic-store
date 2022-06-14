use sea_orm::entity::prelude::TimeDateTimeWithTimeZone;
use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub team_id: Uuid,
    pub name: String,
    pub base_location: String,
    pub default_conversion_profile_id: Uuid,

    pub updated: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::TeamId",
        to = "super::team::Column::Id"
    )]
    Team,
    #[sea_orm(
        belongs_to = "super::conversion_profile::Entity",
        from = "Column::DefaultConversionProfileId",
        to = "super::conversion_profile::Column::Id"
    )]
    ConversionProfile,
    #[sea_orm(has_many = "super::base_image::Entity")]
    BaseImage,
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<super::conversion_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConversionProfile.def()
    }
}

impl Related<super::base_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BaseImage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
