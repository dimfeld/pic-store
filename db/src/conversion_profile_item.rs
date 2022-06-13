use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "conversion_profiles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    #[sea_orm(indexed)]
    pub conversion_profile_id: u64,
    #[sea_orm(indexed)]
    pub user_id: u64,
    #[sea_orm(indexed)]
    pub team_id: u64,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::TeamId",
        to = "super::team::Column::Id"
    )]
    Team,
    #[sea_orm(
        belongs_to = "super::conversion_profile::Entity",
        from = "Column::ConversionProfileId",
        to = "super::conversion_profile::Column::Id"
    )]
    ConversionProfile,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
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

impl ActiveModelBehavior for ActiveModel {}
