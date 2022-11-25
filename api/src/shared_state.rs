use db::object_id::{ProjectId, TeamId, UserId};
use std::sync::Arc;
use uuid::Uuid;

use pic_store_db as db;

use crate::auth::ApiKeyStore;

pub struct InnerState {
    pub production: bool,
    pub db: db::Pool,
    pub queue: prefect::Queue,

    // Hardcoded values until we have real user auth and such.
    pub user_id: UserId,
    pub team_id: TeamId,
    pub project_id: ProjectId,
}

impl std::fmt::Debug for InnerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InnerState")
            .field("production", &self.production)
            .field("user_id", &self.user_id)
            .field("team_id", &self.team_id)
            .field("project_id", &self.project_id)
            .finish_non_exhaustive()
    }
}

pub type AppState = Arc<InnerState>;
