use std::sync::Arc;

use pic_store_auth::RootAuthEvaulator;
use sea_orm::DatabaseConnection;

#[derive(Debug)]
pub struct InnerState {
    pub production: bool,
    pub db: DatabaseConnection,

    pub auth: RootAuthEvaulator,
}

pub type State = Arc<InnerState>;
