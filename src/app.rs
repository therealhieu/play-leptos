use leptos::{html::Header, logging::log, prelude::*, tachys::view::iterators::StaticVec};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{FlatRoutes, Route, RouteProps, Router, Routes},
    StaticSegment,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let config: Config =
        serde_yaml::from_reader(std::fs::File::open("config.yml").unwrap()).unwrap();

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App config/>
            </body>
        </html>
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Text {
    pub text: String,
}

impl Text {
    pub fn render(&self) -> impl IntoView {
        view! {
            <div>{self.text.clone()}</div>
        }
    }
}

#[component]
pub fn App(#[prop(into)] config: Config) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let routes = config
        .pages
        .clone()
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
    let routes = || StaticVec::from(routes);

    log!("config: {:?}", config);

    let titles = config
        .pages
        .iter()
        .map(|page| page.title.clone())
        .collect::<Vec<_>>();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/play-leptos.css"/>
        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <header>
            <For
                each=move || titles.clone()
                key=|title| title.clone()
                children=move |title| {
                    view! {
                      <nav>
                        <a href={format!("/{}", title.clone().to_lowercase())}>{title.clone()}</a>
                      </nav>
                    }
                }
            />
        </header>
        <Router>
            <main>
                <FlatRoutes
                    fallback=|| view! { <div>"Not Found"</div> }
                    children=ToChildren::to_children(routes)
                />

            </main>
        </Router>
    }
}
