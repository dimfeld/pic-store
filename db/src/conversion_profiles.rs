use diesel::{prelude::*, sql_types};
use serde::{Deserialize, Serialize};

pub use crate::schema::conversion_profiles::*;
use crate::{
    diesel_jsonb,
    object_id::{ConversionProfileId, ProjectId, TeamId},
    schema::*,
    ImageFormat,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
pub struct ConversionSize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_aspect_ratio: Option<bool>,
}

diesel_jsonb!(ConversionSize);

// This will eventually contain more details such as format-specific quality settings.
#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
#[serde(tag = "format", rename_all = "lowercase")]
pub enum ConversionFormat {
    Png {
        #[serde(skip_serializing_if = "Option::is_none")]
        condition: Option<FormatConversionCondition>,
    },
    Jpg {
        #[serde(skip_serializing_if = "Option::is_none")]
        condition: Option<FormatConversionCondition>,
    },
    Avif {
        #[serde(skip_serializing_if = "Option::is_none")]
        condition: Option<FormatConversionCondition>,
    },
    Webp {
        #[serde(skip_serializing_if = "Option::is_none")]
        condition: Option<FormatConversionCondition>,
    },
}

diesel_jsonb!(ConversionFormat);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FormatConversionCondition {
    Must { formats: Vec<ImageFormat> },
    MustNot { formats: Vec<ImageFormat> },
}

impl FormatConversionCondition {
    pub fn matches(&self, input_format: ImageFormat) -> bool {
        match self {
            FormatConversionCondition::Must { formats } => formats.contains(&input_format),
            FormatConversionCondition::MustNot { formats } => !formats.contains(&input_format),
        }
    }
}

impl ConversionFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png { .. } => "png",
            Self::Jpg { .. } => "jpg",
            Self::Avif { .. } => "avif",
            Self::Webp { .. } => "webp",
        }
    }

    pub fn matches_condition(&self, input_format: ImageFormat) -> bool {
        let condition = match self {
            Self::Png { condition } => condition.as_ref(),
            Self::Jpg { condition } => condition.as_ref(),
            Self::Avif { condition } => condition.as_ref(),
            Self::Webp { condition } => condition.as_ref(),
        };

        condition.map(|c| c.matches(input_format)).unwrap_or(true)
    }

    pub fn as_db_image_format(&self) -> crate::ImageFormat {
        match self {
            ConversionFormat::Png { .. } => crate::ImageFormat::Png,
            ConversionFormat::Jpg { .. } => crate::ImageFormat::Jpg,
            ConversionFormat::Webp { .. } => crate::ImageFormat::Webp,
            ConversionFormat::Avif { .. } => crate::ImageFormat::Avif,
        }
    }
}

impl From<&ConversionFormat> for crate::ImageFormat {
    fn from(format: &ConversionFormat) -> Self {
        format.as_db_image_format()
    }
}

impl From<&ConversionFormat> for image::ImageFormat {
    fn from(format: &ConversionFormat) -> Self {
        match format {
            ConversionFormat::Png { .. } => image::ImageFormat::Png,
            ConversionFormat::Jpg { .. } => image::ImageFormat::Jpeg,
            ConversionFormat::Webp { .. } => image::ImageFormat::WebP,
            ConversionFormat::Avif { .. } => image::ImageFormat::Avif,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::Jsonb)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConversionOutput {
    Cross {
        formats: Vec<ConversionFormat>,
        sizes: Vec<ConversionSize>,
    },
}

diesel_jsonb!(ConversionOutput);

#[derive(Clone, Debug, Queryable, Identifiable)]
pub struct ConversionProfile {
    pub id: ConversionProfileId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub output: ConversionOutput,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = conversion_profiles)]
pub struct NewConversionProfile {
    pub id: ConversionProfileId,
    pub team_id: TeamId,
    pub project_id: Option<ProjectId>,
    pub name: String,

    pub output: ConversionOutput,
}
