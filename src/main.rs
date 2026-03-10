use dioxus::prelude::*;

use views::{Datasets, DatasetDetail, Home, Navbar, Workspaces, WorkspaceDetail};

mod components;
mod views;

pub mod models;

#[cfg(feature = "server")]
mod db;
mod server;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
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

fn main() {
    #[cfg(feature = "server")]
    {
        db::init_db().expect("Failed to initialize database");
    }
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
