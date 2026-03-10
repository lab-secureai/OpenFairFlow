use crate::Route;
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let mut mobile_open = use_signal(|| false);

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        div { id: "navbar",
            Link { to: Route::Home {}, class: "nav-brand", "OpenFairFlow" }
            div { class: "nav-links",
                Link { to: Route::Home {}, "Home" }
                Link { to: Route::Datasets {}, "Datasets" }
                Link { to: Route::Workspaces {}, "Workspaces" }
            }
            button {
                class: "hamburger",
                onclick: move |_| mobile_open.set(!mobile_open()),
                "☰"
            }
        }

        div { id: "mobile-menu", class: if mobile_open() { "open" } else { "" },
            Link { to: Route::Home {}, onclick: move |_| mobile_open.set(false), "Home" }
            Link {
                to: Route::Datasets {},
                onclick: move |_| mobile_open.set(false),
                "Datasets"
            }
            Link {
                to: Route::Workspaces {},
                onclick: move |_| mobile_open.set(false),
                "Workspaces"
            }
        }

        Outlet::<Route> {}
    }
}
