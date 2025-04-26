use leptos::{
    logging::log, prelude::*, server::codee::string::JsonSerdeCodec,
    tachys::view::iterators::StaticVec,
};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{FlatRoutes, Route, RouteProps, Router},
    StaticSegment,
};
use leptos_use::{use_event_source_with_options, UseEventSourceOptions, UseEventSourceReturn};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Page {
    pub title: String,
    pub path: String,
    pub components: Vec<Component>,
}

impl Page {
    pub fn render(&self) -> impl IntoView {
        let components = self
            .components
            .iter()
            .map(|component| (0, component.clone()))
            .collect::<Vec<_>>();

        view! {
            <For
                each=move || components.clone()
                key=|component| component.0
                children=move |(_i, component)| component.render()
            />
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Component {
    Text(Text),
}

impl Component {
    pub fn render(&self) -> impl IntoView {
        match self {
            Component::Text(text) => text.render(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Text {
    pub text: String,
}

impl Text {
    pub fn render(&self) -> impl IntoView {
        view! { <div>{self.text.clone()}</div> }
    }
}

#[server]
pub async fn get_config() -> Result<Config, ServerFnError> {
    let response = reqwest::get("http://localhost:3000/api/config")
        .await
        .unwrap();
    log!("response: {:?}", response);

    let config = serde_json::from_str::<Config>(&response.text().await.unwrap()).unwrap();

    Ok(config)
}

#[component]
pub fn App() -> impl IntoView {
    let UseEventSourceReturn { data: config, .. } =
        use_event_source_with_options::<Config, JsonSerdeCodec>(
            "http://localhost:3000/api/config-stream",
            UseEventSourceOptions::default().named_events(vec!["config-update".to_string()]),
        );

    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/play-leptos.css" />
        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <header>
            <For
                each=move || {
                    config
                        .get()
                        .unwrap_or_default()
                        .pages
                        .iter()
                        .map(|page| (page.path.clone(), page.title.clone()))
                        .collect::<Vec<_>>()
                }
                key=|(path, title)| format!("{}/{}", path, title)
                children=move |(path, title)| {
                    view! {
                        <nav>
                            <a href=format!("/{}", path.clone())>{title.clone()}</a>
                        </nav>
                    }
                }
            />
        </header>
        <Router>
            <main>
                {move || {
                    log!("config: {:?}", config.get());
                    let routes = config
                        .get()
                        .unwrap_or_default()
                        .pages
                        .into_iter()
                        .map(|page| {
                            let path: &'static str = Box::leak(page.path.clone().into_boxed_str());
                            Route(
                                RouteProps::builder()
                                    .path(StaticSegment(path))
                                    .view(move || page.render())
                                    .build(),
                            )
                        })
                        .collect::<Vec<_>>();
                    let route_defs = || { StaticVec::from(routes) };

                    view! {
                        <FlatRoutes
                            fallback=|| view! { <div>"Not Found"</div> }
                            children=ToChildren::to_children(route_defs)
                        />
                    }
                }}
            </main>
        </Router>
    }
}
