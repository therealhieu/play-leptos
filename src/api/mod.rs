use std::sync::Arc;

use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use tokio::sync::{broadcast, RwLock};

use crate::app::Config;

pub mod config_stream;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub server_state: ServerState,
}

#[derive(Clone)]
pub struct ServerState {
    pub config_change_tx: broadcast::Sender<Config>,
    pub config: Arc<RwLock<Config>>,
}
