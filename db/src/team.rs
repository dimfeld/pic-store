use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "teams")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub deleted: Option<TimeDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user::Entity")]
    User,
    #[sea_orm(has_many = "super::project::Entity")]
    Project,
    #[sea_orm(has_many = "super::base_image::Entity")]
    BaseImage,
    #[sea_orm(has_many = "super::output_image::Entity")]
    OutputImage,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::base_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BaseImage.def()
    }
}

impl Related<super::output_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OutputImage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
