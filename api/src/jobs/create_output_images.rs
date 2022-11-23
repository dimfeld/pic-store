use std::sync::Arc;

use diesel::prelude::*;
use image::DynamicImage;
use prefect::RunningJob;
use serde::{Deserialize, Serialize};

use pic_store_convert as convert;
use pic_store_db as db;
use pic_store_storage as storage;

use crate::Result;
use db::{
    base_images,
    conversion_profiles::{ConversionFormat, ConversionSize},
    object_id::{BaseImageId, OutputImageId},
    storage_locations::Provider,
    upload_profiles, BaseImageStatus, OutputImageStatus, PoolExt,
};
use tracing::{event, instrument, Level};

use super::JobContext;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateOutputImagesJobPayload {
    pub base_image: BaseImageId,
    pub conversions: Vec<OutputImageId>,
}

#[instrument(skip(job))]
pub async fn create_output_images_job(
    job: RunningJob,
    context: JobContext,
) -> Result<(), anyhow::Error> {
    let mut payload = job.json_payload::<CreateOutputImagesJobPayload>()?;

    event!(Level::INFO, ?payload);

    let (bst, ost) = diesel::alias!(db::storage_locations as bst, db::storage_locations as ost);

    let (
        base_image_location,
        base_image_base_location,
        base_image_storage_provider,
        output_image_base_location,
        output_image_storage_provider,
    ) = context
        .pool
        .interact(move |conn| {
            db::base_images::table
                .inner_join(
                    db::upload_profiles::table
                        .on(base_images::upload_profile_id.eq(upload_profiles::id))
                        .inner_join(
                            bst.on(db::upload_profiles::base_storage_location_id
                                .eq(bst.field(db::storage_locations::id))),
                        )
                        .inner_join(
                            ost.on(db::upload_profiles::output_storage_location_id
                                .eq(ost.field(db::storage_locations::id))),
                        ),
                )
                .filter(db::base_images::id.eq(payload.base_image))
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
    let base_image = read_image(
        base_image_storage,
        base_image_base_location.as_str(),
        base_image_location.as_str(),
    )
    .await?;

    let output_image_storage = storage::Provider::from_db(output_image_storage_provider)?;
    let output_operator = output_image_storage
        .create_operator(output_image_base_location.as_str())
        .await?;

    while let Some(output_image_id) = payload.conversions.pop() {
        //  Get the next conversion profile from the list
        let (output_location, conversion_format, conversion_size) = context
            .pool
            .interact(move |conn| {
                diesel::update(db::output_images::table)
                    .filter(db::output_images::id.eq(output_image_id))
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

        event!(Level::INFO, image=%output_location, "Converting image");
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
        let convert_result =
            tokio::task::spawn_blocking(move || convert::convert(&b, output_format, &size))
                .await??;

        output_operator
            .object(output_location.as_str())
            .write(convert_result.image)
            .await?;

        context
            .pool
            .interact(move |conn| {
                // Add the OutputImage entry
                diesel::update(db::output_images::table)
                    .filter(db::output_images::id.eq(output_image_id))
                    .set((
                        db::output_images::status.eq(OutputImageStatus::Ready),
                        db::output_images::width.eq(convert_result.width as i32),
                        db::output_images::height.eq(convert_result.height as i32),
                    ))
                    .execute(conn)?;

                Ok::<_, anyhow::Error>(())
            })
            .await?;

        job.checkpoint_json(&payload).await?;
    }

    // Set the base image status to done.
    context
        .pool
        .interact(move |conn| {
            diesel::update(db::base_images::table)
                .filter(db::base_images::id.eq(payload.base_image))
                .set(db::base_images::status.eq(BaseImageStatus::Ready))
                .execute(conn)?;

            Ok::<_, anyhow::Error>(())
        })
        .await?;

    Ok(())
}

async fn read_image(
    storage_provider: pic_store_storage::Provider,
    base_location: &str,
    location: &str,
) -> Result<Arc<DynamicImage>, anyhow::Error> {
    let op = storage_provider.create_operator(base_location).await?;
    let base_image_data = op.object(location).read().await?;
    let base_image = Arc::new(convert::image_from_bytes(base_image_data.as_slice())?);
    Ok(base_image)
}
