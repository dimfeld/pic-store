use sea_orm::entity::prelude::TimeDateTime;
use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "output_images")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    #[sea_orm(indexed)]
    pub team_id: Uuid,
    pub base_image_id: Uuid,
    pub hash: String,
    pub location: String,
    pub width: u32,
    pub height: u32,
    pub format: String,

    pub status: OutputImageStatus,

    pub created: TimeDateTime,
    pub deleted: Option<TimeDateTimeWithTimeZone>,
}

#[derive(EnumIter, DeriveActiveEnum, PartialEq, Eq, Copy, Clone, Debug)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "storage_provider")]
pub enum OutputImageStatus {
    #[sea_orm(string_value = "queued")]
    Queued,
    #[sea_orm(string_value = "converting")]
    Converting,
    #[sea_orm(string_value = "ready")]
    Ready,
    #[sea_orm(string_value = "queued_for_delete")]
    QueuedForDelete,
    #[sea_orm(string_value = "deleted")]
    Deleted,
}

impl Default for OutputImageStatus {
    fn default() -> Self {
        Self::Queued
    }
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
        belongs_to = "super::base_image::Entity",
        from = "Column::BaseImageId",
        to = "super::base_image::Column::Id"
    )]
    BaseImage,
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

impl Related<super::base_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BaseImage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
