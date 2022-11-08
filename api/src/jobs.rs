pub mod create_output_images;

use std::path::Path;

pub use create_output_images::*;

use pic_store_db as db;
use prefect::{JobRunner, Queue, Worker};
use tracing::{event, Level};

#[derive(Clone)]
pub struct JobContext {
    pub pool: db::Pool,
}

impl std::fmt::Debug for JobContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobContext").finish_non_exhaustive()
    }
}

pub const CREATE_OUTPUT_IMAGES: &str = "create_output_images";

pub async fn create_job_queue(
    db_path: &Path,
    pool: db::Pool,
) -> Result<(Queue, Worker), prefect::Error> {
    event!(Level::INFO, "Starting background worker task");
    let queue = Queue::new(db_path).await?;
    let context = JobContext { pool };

    let create_output_images =
        JobRunner::builder(CREATE_OUTPUT_IMAGES, create_output_images_job).build();

    let worker = Worker::builder(&queue, context)
        .jobs([create_output_images])
        .max_concurrency(10)
        .build()
        .await?;

    Ok((queue, worker))
}
