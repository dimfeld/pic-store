use sea_orm::entity::prelude::TimeDateTimeWithTimeZone;
use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "base_images")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    #[sea_orm(indexed)]
    pub team_id: Uuid,
    #[sea_orm(indexed)]
    pub project_id: Uuid,
    #[sea_orm(indexed)]
    pub hash: String,

    /// The original filename of the image.
    pub filename: String,

    /// A subpath at which the image can be found, with the linked StorageLocation in the
    /// UploadSettings as the root location.
    pub location: String,

    pub width: u32,
    pub height: u32,
    pub format: String,

    pub upload_settings_id: Uuid,
    pub alt_text: String,
    pub placeholder: String,

    pub updated: TimeDateTimeWithTimeZone,
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
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
    #[sea_orm(
        belongs_to = "super::upload_settings::Entity",
        from = "Column::UploadSettingsId",
        to = "super::upload_settings::Column::Id"
    )]
    UploadSettings,
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

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::upload_settings::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UploadSettings.def()
    }
}

impl Related<super::output_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OutputImage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
