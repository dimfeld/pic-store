use crate::conversion_profiles::ConversionFormat;
use crate::enums::{ImageFormat, OutputImageStatus};
use crate::object_id::{BaseImageId, ConversionProfileItemId, OutputImageId, TeamId};
use diesel::prelude::*;

pub use crate::schema::output_images::*;

use crate::schema::*;

#[derive(Clone, Debug, Queryable, Insertable, Identifiable)]
#[diesel(primary_key(output_image_id))]
pub struct OutputImage {
    pub output_image_id: OutputImageId,
    pub team_id: TeamId,
    pub base_image_id: BaseImageId,
    pub location: String,
    pub width: i32,
    pub height: i32,
    pub format: ConversionFormat,

    pub status: OutputImageStatus,

    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = output_images)]
pub struct NewOutputImage {
    pub output_image_id: OutputImageId,
    pub team_id: TeamId,
    pub base_image_id: BaseImageId,
    pub location: String,
    pub width: i32,
    pub height: i32,
    pub format: ConversionFormat,

    pub status: OutputImageStatus,
}
