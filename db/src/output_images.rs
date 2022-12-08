use diesel::prelude::*;

pub use crate::schema::output_images::*;
use crate::{
    conversion_profiles::{ConversionFormat, ConversionSize},
    enums::OutputImageStatus,
    object_id::{BaseImageId, OutputImageId, TeamId},
    schema::*,
};

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
pub struct OutputImage {
    pub id: OutputImageId,
    pub team_id: TeamId,
    pub base_image_id: BaseImageId,
    pub location: String,
    pub file_size: i32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: ConversionSize,
    pub format: ConversionFormat,

    pub status: OutputImageStatus,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = output_images)]
pub struct NewOutputImage {
    pub id: OutputImageId,
    pub team_id: TeamId,
    pub base_image_id: BaseImageId,
    pub location: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: ConversionSize,
    pub format: ConversionFormat,

    pub status: OutputImageStatus,
}
