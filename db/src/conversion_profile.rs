use sea_orm::entity::prelude::TimeDateTimeWithTimeZone;
use sea_orm::{prelude::*, Set};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "conversion_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub team_id: Uuid,
    pub name: String,
    pub updated: TimeDateTimeWithTimeZone,
    pub deleted: Option<TimeDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::TeamId",
        to = "super::team::Column::Id"
    )]
    Team,
    #[sea_orm(has_many = "super::upload_profile::Entity")]
    UploadProfile,
    #[sea_orm(has_many = "super::conversion_profile_item::Entity")]
    ConversionProfileItem,
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<super::upload_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UploadProfile.def()
    }
}

impl Related<super::conversion_profile_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConversionProfileItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::new_v4()),
            updated: Set(TimeDateTimeWithTimeZone::now_utc()),
            ..ActiveModelTrait::default()
        }
    }
}
