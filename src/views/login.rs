use crate::server::login_server;
use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn Login() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);
    let nav = use_navigator();

    let on_submit = move |e: Event<FormData>| {
        e.prevent_default();
        let nav = nav;
        spawn(async move {
            loading.set(true);
            error.set(None);

            match login_server(username(), password()).await {
                Ok(true) => {
                    nav.push(Route::Home {});
                }
                Ok(false) => {
                    error.set(Some("Invalid username or password".to_string()));
                }
                Err(e) => {
                    error.set(Some(format!("Login error: {e}")));
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen bg-black flex items-center justify-center px-4",
            div { class: "w-full max-w-sm",
                div { class: "text-center mb-8",
                    h1 { class: "text-3xl font-bold text-white tracking-tight", "OpenFairFlow" }
                    p { class: "text-[#888] mt-2", "Sign in to continue" }
                }

                form {
                    class: "border border-[#2a2a2a] rounded-xl p-6 space-y-5",
                    onsubmit: on_submit,

                    if let Some(err) = error() {
                        div { class: "bg-red-500/10 border border-red-500/30 rounded-lg px-4 py-3 text-sm text-red-400",
                            "{err}"
                        }
                    }

                    div {
                        label { class: "block text-xs font-medium text-[#888] uppercase tracking-wider mb-2",
                            "Username"
                        }
                        input {
                            class: "w-full bg-[#111] border border-[#2a2a2a] rounded-lg px-4 py-2.5 text-white text-sm placeholder-[#555] focus:border-[#444] focus:outline-none transition-colors",
                            r#type: "text",
                            placeholder: "Enter username",
                            required: true,
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                    }

                    div {
                        label { class: "block text-xs font-medium text-[#888] uppercase tracking-wider mb-2",
                            "Password"
                        }
                        input {
                            class: "w-full bg-[#111] border border-[#2a2a2a] rounded-lg px-4 py-2.5 text-white text-sm placeholder-[#555] focus:border-[#444] focus:outline-none transition-colors",
                            r#type: "password",
                            placeholder: "Enter password",
                            required: true,
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }

                    button {
                        class: "w-full bg-white text-black font-medium rounded-lg py-2.5 text-sm hover:bg-[#ddd] transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                        r#type: "submit",
                        disabled: loading(),
                        if loading() {
                            "Signing in…"
                        } else {
                            "Sign in"
                        }
                    }
                }
            }
        }
    }
}
