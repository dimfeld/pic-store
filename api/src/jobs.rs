pub mod create_output_images;

pub use create_output_images::*;

use sqlxmq::{JobRegistry, JobRunnerHandle};

use pic_store_db as db;
use tracing::{event, Level};

#[derive(Clone)]
pub struct JobContext {
    pub pool: db::Pool,
}

pub fn create_registry(pool: db::Pool) -> JobRegistry {
    let mut registry = JobRegistry::new(&[&create_output_images::create_output_images_job]);
    registry.set_context(JobContext { pool });
    registry
}

pub async fn start_workers(
    database_url: String,
    pool: db::Pool,
    registry: JobRegistry,
    min_concurrency: usize,
    max_concurrency: usize,
) -> Result<JobRunnerHandle, sqlxmq::Error> {
    event!(Level::INFO, "Starting background worker task");
    registry
        .runner(pool, database_url)
        .set_channel_names(&["images"])
        .set_concurrency(min_concurrency, max_concurrency)
        .run()
        .await
}
