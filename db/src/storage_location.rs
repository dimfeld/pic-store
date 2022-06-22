use std::fmt::Display;

use sea_orm::entity::prelude::TimeDateTimeWithTimeZone;
use sea_orm::prelude::*;

#[derive(EnumIter, DeriveActiveEnum, PartialEq, Eq, Copy, Clone, Debug)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "storage_provider")]
pub enum Provider {
    /// Local filesystem
    #[sea_orm(string_value = "local")]
    Local,
    /// S3 or compatible storage
    #[sea_orm(string_value = "s3")]
    S3,
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Local => "Local",
            Self::S3 => "S3",
        };

        f.write_str(label)
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "storage_locations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub project_id: Uuid,
    pub name: String,

    pub provider: Provider,
    pub base_location: String,
    pub credentials: Option<serde_json::Value>,

    /// The base URL at which images in this StorageLocation can be accessed on the web.
    pub public_url_base: String,

    pub updated: TimeDateTimeWithTimeZone,
    pub deleted: Option<TimeDateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
