use std::sync::Arc;

use anyhow::anyhow;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use sqlxmq::{job, Checkpoint, CurrentJob};

use pic_store_convert as convert;
use pic_store_db as db;
use pic_store_storage as storage;

use crate::Error;
use db::{
    conversion_profiles::{ConversionFormat, ConversionSize},
    object_id::{BaseImageId, ConversionProfileId, OutputImageId},
    output_images::NewOutputImage,
    storage_locations::Provider,
    BaseImageStatus, ImageFormat, OutputImageStatus, PoolExt,
};
use tracing::{event, instrument, Level};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateOutputImagesJobPayload {
    pub base_image: BaseImageId,
    pub conversions: Vec<OutputImageId>,
}

#[job(channel_name = "images")]
#[instrument]
pub async fn create_output_images_job(mut current_job: CurrentJob) -> Result<(), anyhow::Error> {
    let mut payload = current_job
        .json::<CreateOutputImagesJobPayload>()?
        .ok_or_else(|| anyhow!("Missing payload"))?;

    event!(Level::INFO, ?payload);

    let pool = current_job.pool().clone();

    let (bst, ost) = diesel::alias!(db::storage_locations as bst, db::storage_locations as ost);

    let (
        base_image_location,
        base_image_base_location,
        base_image_storage_provider,
        output_image_base_location,
        output_image_storage_provider,
    ) = pool
        .interact(move |conn| {
            db::base_images::table
                .inner_join(
                    db::upload_profiles::table
                        .inner_join(
                            bst.on(db::upload_profiles::base_storage_location_id
                                .eq(bst.field(db::storage_locations::storage_location_id))),
                        )
                        .inner_join(
                            ost.on(db::upload_profiles::base_storage_location_id
                                .eq(ost.field(db::storage_locations::storage_location_id))),
                        ),
                )
                .filter(db::base_images::base_image_id.eq(payload.base_image))
                .select((
                    db::base_images::location,
                    bst.field(db::storage_locations::base_location),
                    bst.field(db::storage_locations::provider),
                    ost.field(db::storage_locations::base_location),
                    ost.field(db::storage_locations::provider),
                ))
                .first::<(String, String, Provider, String, Provider)>(conn)
                .map_err(anyhow::Error::new)
        })
        .await?;

    let base_image_storage = storage::Provider::from_db(base_image_storage_provider)?;
    let op = base_image_storage
        .create_operator(base_image_base_location.as_str())
        .await?;
    let base_image_data = op.object(base_image_location.as_str()).read().await?;
    let base_image = Arc::new(convert::image_from_bytes(base_image_data.as_slice())?);

    let output_image_storage = storage::Provider::from_db(output_image_storage_provider)?;
    let output_operator = output_image_storage
        .create_operator(output_image_base_location.as_str())
        .await?;

    while let Some(output_image_id) = payload.conversions.pop() {
        //  Get the next conversion profile from the list
        let (output_location, conversion_format, conversion_size) = pool
            .interact(move |conn| {
                diesel::update(db::output_images::table)
                    .filter(db::output_images::output_image_id.eq(output_image_id))
                    .set(db::output_images::status.eq(OutputImageStatus::Converting))
                    .returning((
                        db::output_images::location,
                        db::output_images::format,
                        db::output_images::size,
                    ))
                    .get_result::<(String, ConversionFormat, ConversionSize)>(conn)
                    .map_err(anyhow::Error::new)
            })
            .await?;

        //  Do the conversion

        let size = convert::ImageSizeTransform {
            width: conversion_size.width,
            height: conversion_size.height,
            preserve_aspect_ratio: conversion_size.preserve_aspect_ratio.unwrap_or(true),
        };

        let output_format = match conversion_format {
            ConversionFormat::Png => image::ImageFormat::Png,
            ConversionFormat::Jpg => image::ImageFormat::Jpeg,
            ConversionFormat::Webp => image::ImageFormat::WebP,
            ConversionFormat::Avif => image::ImageFormat::Avif,
        };

        let b = base_image.clone();
        let output =
            tokio::task::spawn_blocking(move || convert::convert(&b, output_format, &size))
                .await??;

        output_operator
            .object(output_location.as_str())
            .write(&output)
            .await?;

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
