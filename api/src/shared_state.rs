use std::sync::Arc;

use pic_store_auth::RootAuthEvaulator;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

#[derive(Debug)]
pub struct InnerState {
    pub production: bool,
    pub db: DatabaseConnection,

    pub auth: RootAuthEvaulator,

    // Hardcoded values until we have real user auth and such.
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub project_id: Uuid,
}

pub type State = Arc<InnerState>;
