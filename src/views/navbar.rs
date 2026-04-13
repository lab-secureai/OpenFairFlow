use crate::Route;
use crate::server::{check_auth_server, logout_server};
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let mut mobile_open = use_signal(|| false);
    let nav = use_navigator();

    // Client-side auth guard: redirect to /login if not authenticated
    let auth = use_server_future(check_auth_server)?;
    if let Some(Ok(false)) = auth() {
        nav.push(Route::Login {});
        return rsx! {};
    }

    let on_logout = move |_| {
        let nav = nav;
        spawn(async move {
            let _ = logout_server().await;
            nav.push(Route::Login {});
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        div { id: "navbar",
            Link { to: Route::Home {}, class: "nav-brand", "OpenFairFlow" }
            div { class: "nav-links",
                Link { to: Route::Home {}, "Home" }
                Link { to: Route::Datasets {}, "Datasets" }
                Link { to: Route::Workspaces {}, "Workspaces" }
                button { class: "nav-logout", onclick: on_logout, "Logout" }
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
            button { class: "nav-logout", onclick: on_logout, "Logout" }
        }

        Outlet::<Route> {}
    }
}
