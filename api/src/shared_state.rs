use std::sync::Arc;

use sea_orm::DatabaseConnection;

#[derive(Debug)]
pub struct InnerState {
    pub production: bool,
    pub db: DatabaseConnection,
}

pub type State = Arc<InnerState>;
