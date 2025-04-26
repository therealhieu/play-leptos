#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::sync::Arc;

    use axum::{extract::State, routing::get, Json, Router};
    use leptos::{config::get_configuration, logging::log};
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use play_leptos::{
        api::{
            config_stream::{config_stream_handler, start_config_watcher},
            AppState, ServerState,
        },
        app::*,
    };
    use tokio::sync::{broadcast, RwLock};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let inital_config =
        serde_yaml::from_str(std::fs::read_to_string("config.yml").unwrap().as_str()).unwrap();
    let (tx, _) = broadcast::channel(16);
    let server_state = ServerState {
        config_change_tx: tx.clone(),
        config: Arc::new(RwLock::new(inital_config)),
    };

    let _config_watcher = start_config_watcher(
        tx.clone(),
        server_state.config.clone(),
        "config.yml".to_string(),
    )
    .await
    .unwrap();
    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        server_state,
    };

    let app = Router::new()
        .leptos_routes(&app_state, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .route(
            "/api/config",
            get(|State(state): State<AppState>| async move {
                Json(state.server_state.config.read().await.clone())
            }),
        )
        .route("/api/config-stream", get(config_stream_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .with_state(app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
