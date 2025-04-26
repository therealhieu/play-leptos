use std::{error::Error, path::Path, sync::Arc, time::Duration};

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::stream::Stream;
use leptos::logging::{error, log, warn};
use notify::{
    event::{AccessKind, AccessMode},
    EventKind, RecursiveMode, Watcher,
};
use tokio::{
    runtime::Handle,
    sync::{broadcast, RwLock},
};

use crate::app::Config;

use super::AppState;
pub async fn start_config_watcher(
    tx: broadcast::Sender<Config>,
    config: Arc<RwLock<Config>>,
    path: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let watcher_path = path.clone();
    log!("Loading initial config from {}", &watcher_path);
    let initial_content = tokio::fs::read_to_string(&watcher_path).await?;
    let initial_config: Config = serde_yaml::from_str(&initial_content)?;
    *config.write().await = initial_config;
    log!("Initial config loaded for {}", watcher_path);

    let tokio_handle = Handle::current();

    let watcher_tx = tx;
    let latest_config = config;

    // Spawn Background Task for Watching
    tokio::task::spawn(async move {
        log!("Starting file watcher task for: {}", watcher_path);

        // Clone the handle again to move into the notify callback closure
        let callback_tokio_handle = tokio_handle.clone();
        let callback_tx = watcher_tx.clone();
        let callback_latest_config = latest_config.clone();
        let callback_path = watcher_path.clone();

        // Create the watcher *inside* the background task
        let mut watcher =
            match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                // Clone resources *again* for the inner task spawned per event
                // *** Importantly, clone the Tokio handle too! ***
                let inner_task_tokio_handle = callback_tokio_handle.clone();
                let inner_tx = callback_tx.clone();
                let inner_latest_config = callback_latest_config.clone();
                let inner_path = callback_path.clone();

                match res {
                    Ok(event) => {
                        if event.kind.is_modify()
                            || event.kind == EventKind::Access(AccessKind::Close(AccessMode::Write))
                        {
                            inner_task_tokio_handle.spawn(async move {
                                let content = match tokio::fs::read_to_string(&inner_path).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        error!(
                                            "Watcher task: Failed to read {}: {:?}",
                                            inner_path, e
                                        );
                                        return;
                                    }
                                };

                                // Parse Config (with error handling)
                                let config = match serde_yaml::from_str::<Config>(&content) {
                                    Ok(cfg) => cfg,
                                    Err(e) => {
                                        error!(
                                            "Watcher task: Failed to parse {}: {:?}",
                                            inner_path, e
                                        );
                                        return;
                                    }
                                };

                                // Update State & Broadcast
                                log!("Watcher task: Parsed update for {}", inner_path);
                                *inner_latest_config.write().await = config.clone();
                                let _ = inner_tx.send(config);
                            });
                        }
                    }
                    Err(e) => {
                        error!("File watch error event: {:?}", e);
                    }
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    error!(
                        "Failed to create file watcher: {:?}. Watcher task exiting.",
                        e
                    );
                    return;
                }
            }; // watcher is owned by the task scope

        if let Err(e) = watcher.watch(Path::new(&watcher_path), RecursiveMode::NonRecursive) {
            error!(
                "Failed to start watching {}: {:?}. Watcher task exiting.",
                watcher_path, e
            );
            return;
        } else {
            log!("Successfully watching file: {}", watcher_path);
        }

        // Keep this background task alive
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });

    log!("start_config_watcher function finished, background task spawned.");
    Ok(())
}

pub async fn config_stream_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, axum::BoxError>>> {
    let server_state = state.server_state;
    let mut rx = server_state.config_change_tx.subscribe();

    let stream = async_stream::stream! {
        let inital_config = server_state.config.read().await.clone();
        let config_json = serde_json::to_string(&inital_config)?;
        yield Ok(Event::default().data(config_json).event("config-update"));

        loop {
            match rx.recv().await {
                Ok(config) => {
                    let config_json = serde_json::to_string(&config)?;
                    log!("config_json: {}", config_json);
                    yield Ok(Event::default().data(config_json).event("config-update"));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Config change stream closed or lagged too much");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Config change stream lagged by {} events", n);
                    let config = server_state.config.read().await.clone();
                    let config_json = serde_json::to_string(&config)?;
                    yield Ok(Event::default().data(config_json).event("config-update"));
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(1)))
}
