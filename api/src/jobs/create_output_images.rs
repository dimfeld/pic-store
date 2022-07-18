use anyhow::anyhow;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use sqlxmq::{job, Checkpoint, CurrentJob};

use pic_store_db as db;
use pic_store_storage as storage;

use crate::Error;
use db::{
    conversion_profiles::ConversionFormat,
    object_id::{BaseImageId, ConversionProfileId, OutputImageId},
    output_images::NewOutputImage,
    BaseImageStatus, OutputImageStatus, PoolExt,
};
use tracing::{event, instrument, Level};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateOutputImagesJobPayload {
    base_image: BaseImageId,
    conversions: Vec<OutputImageId>,
}

#[job(channel_name = "images")]
#[instrument]
pub async fn create_output_images_job(mut current_job: CurrentJob) -> Result<(), anyhow::Error> {
    let mut payload = current_job
        .json::<CreateOutputImagesJobPayload>()?
        .ok_or_else(|| anyhow!("Missing payload"))?;

    event!(Level::INFO, ?payload);

    let pool = current_job.pool().clone();

    while let Some(output_image_id) = payload.conversions.pop() {
        //  Get the next conversion profile from the list
        let (conversion_format, width, height) = pool
            .interact(move |conn| {
                diesel::update(db::output_images::table)
                    .filter(db::output_images::output_image_id.eq(output_image_id))
                    .set(db::output_images::status.eq(OutputImageStatus::Converting))
                    .returning((
                        db::output_images::format,
                        db::output_images::width,
                        db::output_images::height,
                    ))
                    .get_result::<(ConversionFormat, i32, i32)>(conn)
                    .map_err(anyhow::Error::new)
            })
            .await?;

        //  Do the conversion
        //  Write the file to storage

        let payload = payload.clone();
        current_job = pool
            .interact(move |conn| {
                //   Add the OutputImage entry
                diesel::update(db::output_images::table)
                    .filter(db::output_images::output_image_id.eq(output_image_id))
                    .set(db::output_images::status.eq(OutputImageStatus::Ready))
                    .execute(conn)?;

                //   Write a checkpoint
                let mut cp = Checkpoint::new();
                cp.set_json(&payload)?;
                current_job.checkpoint(conn, &cp)?;

                Ok::<_, anyhow::Error>(current_job)
            })
            .await?;
    }

    // Set the base image status to done.
    pool.interact(move |conn| {
        diesel::update(db::base_images::table)
            .filter(db::base_images::base_image_id.eq(payload.base_image))
            .set(db::base_images::status.eq(BaseImageStatus::Ready))
            .execute(conn)?;

        Ok::<_, anyhow::Error>(())
    })
    .await?;

    Ok(())
}
