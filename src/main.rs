use dioxus::prelude::*;

use views::{DatasetDetail, Datasets, Home, Login, Navbar, WorkspaceDetail, Workspaces};

mod components;
mod views;

pub mod models;

#[cfg(feature = "server")]
mod db;
mod server;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/login")]
    Login {},
    #[layout(Navbar)]
        #[route("/")]
        Home {},
        #[route("/datasets")]
        Datasets {},
        #[route("/datasets/:id")]
        DatasetDetail { id: String },
        #[route("/workspaces")]
        Workspaces {},
        #[route("/workspaces/:id")]
        WorkspaceDetail { id: String },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");

const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use dioxus::prelude::dioxus_server::DioxusRouterExt;

    db::init_db().expect("Failed to initialize database");
    let _ = dotenvy::dotenv();

    let address = dioxus::cli_config::fullstack_address_or_localhost();

    let router = axum::Router::new()
        .serve_dioxus_application(dioxus_server::ServeConfig::new(), App)
        .layer(axum::middleware::from_fn(server::auth::auth_middleware));

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        Router::<Route> {}
    }
}
