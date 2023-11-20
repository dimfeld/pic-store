use std::sync::Arc;

use bytes::Bytes;
use db::{
    base_images,
    conversion_profiles::{ConversionFormat, ConversionSize},
    image_base_location,
    object_id::{BaseImageId, OutputImageId},
    storage_locations::Provider,
    upload_profiles, BaseImageStatus, OutputImageStatus, PoolExt,
};
use diesel::prelude::*;
use effectum::RunningJob;
use image::DynamicImage;
use pic_store_convert as convert;
use pic_store_db as db;
use pic_store_storage as storage;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

use super::JobContext;
use crate::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateOutputImagesJobPayload {
    pub base_image: BaseImageId,
    pub conversions: Vec<OutputImageId>,
}

#[instrument(skip(job))]
pub async fn create_output_images_job(
    job: RunningJob,
    context: JobContext,
) -> Result<(), eyre::Report> {
    let mut payload = job.json_payload::<CreateOutputImagesJobPayload>()?;

    event!(Level::INFO, ?payload);

    let (bst, ost) = diesel::alias!(db::storage_locations as bst, db::storage_locations as ost);

    let (
        project_base_location,
        base_image_location,
        base_image_base_location,
        base_image_profile_base_path,
        base_image_storage_provider,
        output_image_base_location,
        output_image_profile_base_path,
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
                .inner_join(
                    db::projects::table.on(db::projects::id.eq(db::base_images::project_id)),
                )
                .filter(db::base_images::id.eq(payload.base_image))
                .select((
                    db::projects::base_location,
                    db::base_images::location,
                    bst.field(db::storage_locations::base_location),
                    upload_profiles::base_storage_location_path,
                    bst.field(db::storage_locations::provider),
                    ost.field(db::storage_locations::base_location),
                    upload_profiles::output_storage_location_path,
                    ost.field(db::storage_locations::provider),
                ))
                .first::<(
                    String,
                    String,
                    String,
                    Option<String>,
                    Provider,
                    String,
                    Option<String>,
                    Provider,
                )>(conn)
                .map_err(eyre::Report::new)
        })
        .await?;

    let base_image_base_location = image_base_location(
        &base_image_base_location,
        &project_base_location,
        &base_image_profile_base_path,
    );

    let base_image_storage = storage::Provider::from_db(base_image_storage_provider)?;
    let base_image = read_image(
        base_image_storage,
        base_image_base_location.as_ref(),
        base_image_location.as_str(),
    )
    .await?;

    let output_image_base_location = image_base_location(
        &output_image_base_location,
        &project_base_location,
        &output_image_profile_base_path,
    );

    let output_image_storage = storage::Provider::from_db(output_image_storage_provider)?;
    let output_operator = output_image_storage
        .create_operator(output_image_base_location.as_ref())
        .await?;

    while let Some(output_image_id) = payload.conversions.pop() {
        //  Get the next conversion profile from the list
        let (output_location, conversion_format, conversion_size) = context
            .pool
            .interact(move |conn| {
                diesel::update(db::output_images::table)
                    .filter(db::output_images::id.eq(output_image_id))
                    .set((
                        db::output_images::status.eq(OutputImageStatus::Converting),
                        db::output_images::updated.eq(diesel::dsl::now),
                    ))
                    .returning((
                        db::output_images::location,
                        db::output_images::format,
                        db::output_images::size,
                    ))
                    .get_result::<(String, ConversionFormat, ConversionSize)>(conn)
                    .map_err(eyre::Report::new)
            })
            .await?;

        //  Do the conversion

        event!(Level::INFO, image=%output_location, "Converting image");
        let size = convert::ImageSizeTransform {
            width: conversion_size.width,
            height: conversion_size.height,
            preserve_aspect_ratio: conversion_size.preserve_aspect_ratio.unwrap_or(true),
        };

        let output_format = image::ImageFormat::from(&conversion_format);
        let quality = conversion_format.quality();
        let b = base_image.clone();
        let convert_result = tokio::task::spawn_blocking(move || {
            convert::convert(&b, output_format, quality, &size)
        })
        .await??;

        let size_bytes = convert_result.image.len() as i32;
        output_operator
            .put(output_location.as_str(), Bytes::from(convert_result.image))
            .await?;

        context
            .pool
            .interact(move |conn| {
                // Add the OutputImage entry
                diesel::update(db::output_images::table)
                    .filter(db::output_images::id.eq(output_image_id))
                    .set((
                        db::output_images::status.eq(OutputImageStatus::Ready),
                        db::output_images::file_size.eq(size_bytes),
                        db::output_images::width.eq(convert_result.width as i32),
                        db::output_images::height.eq(convert_result.height as i32),
                        db::output_images::updated.eq(diesel::dsl::now),
                    ))
                    .execute(conn)?;

                Ok::<_, eyre::Report>(())
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

            Ok::<_, eyre::Report>(())
        })
        .await?;

    Ok(())
}

async fn read_image(
    storage_provider: pic_store_storage::Provider,
    base_location: &str,
    location: &str,
) -> Result<Arc<DynamicImage>, eyre::Report> {
    let op = storage_provider.create_operator(base_location).await?;
    let base_image_data = op.get(location).await?;
    let buffer = base_image_data.bytes().await?;
    let base_image = Arc::new(convert::image_from_bytes(&buffer)?);
    Ok(base_image)
}
