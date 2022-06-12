use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "base_images")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    #[sea_orm(indexed)]
    pub user_id: u64,
    #[sea_orm(indexed)]
    pub team_id: u64,
    #[sea_orm(indexed)]
    pub hash: String,
    pub filename: String,
    pub location: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
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
    #[sea_orm(has_many = "super::output_image::Entity")]
    OutputImage,
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

impl Related<super::output_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OutputImage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
