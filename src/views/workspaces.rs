use dioxus::prelude::*;
use crate::Route;
use crate::server::{list_workspaces_server, create_workspace_server, delete_workspace_server, list_datasets_server};

const WORKSPACES_CSS: Asset = asset!("/assets/styling/workspaces.css");

#[component]
pub fn Workspaces() -> Element {
    let mut show_form = use_signal(|| false);
    let mut workspaces = use_server_future(move || list_workspaces_server())?;
    let datasets = use_resource(move || list_datasets_server());

    // Form state
    let mut ws_name = use_signal(|| String::new());
    let mut ws_dataset_id = use_signal(|| String::new());
    let mut creating = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let handle_create = move |_| {
        async move {
            let name = ws_name().trim().to_string();
            let dataset_id = ws_dataset_id().trim().to_string();

            if name.is_empty() {
                error_msg.set(Some("Name is required".to_string()));
                return;
            }
            if dataset_id.is_empty() {
                error_msg.set(Some("Please select a dataset".to_string()));
                return;
            }

            creating.set(true);
            error_msg.set(None);

            match create_workspace_server(name, dataset_id).await {
                Ok(_) => {
                    ws_name.set(String::new());
                    ws_dataset_id.set(String::new());
                    show_form.set(false);
                    workspaces.restart();
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to create workspace: {e}")));
                }
            }
            creating.set(false);
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: WORKSPACES_CSS }

        div { class: "max-w-7xl mx-auto px-4 py-8",
            // Header
            div { class: "flex items-center justify-between mb-8",
                h1 { class: "text-3xl font-bold tracking-tight text-white", "Workspaces" }
                button {
                    class: if show_form() { "border border-[#444] text-[#888] hover:text-white hover:border-[#666] font-medium py-2 px-4 rounded-lg transition-colors" } else { "bg-white text-black hover:bg-[#ddd] font-medium py-2 px-4 rounded-lg transition-colors" },
                    onclick: move |_| {
                        show_form.set(!show_form());
                        error_msg.set(None);
                    },
                    if show_form() {
                        "Cancel"
                    } else {
                        "+ New Workspace"
                    }
                }
            }

            // Create Workspace Form
            if show_form() {
                div { class: "card-surface rounded-xl p-6 mb-8",
                    h2 { class: "text-xl font-semibold text-white mb-4", "Create Workspace" }

                    div { class: "space-y-4",
                        // Name
                        div {
                            label { class: "block text-sm text-[#888] mb-1", "Workspace Name" }
                            input {
                                r#type: "text",
                                class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white focus:border-[#555] focus:outline-none",
                                placeholder: "My Analysis",
                                value: "{ws_name}",
                                oninput: move |e| ws_name.set(e.value()),
                            }
                        }

                        // Dataset selector
                        div {
                            label { class: "block text-sm text-[#888] mb-1", "Dataset" }
                            match datasets() {
                                Some(Ok(list)) => {
                                    rsx! {
                                        select {
                                            class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white focus:border-[#555] focus:outline-none",
                                            value: "{ws_dataset_id}",
                                            onchange: move |e| ws_dataset_id.set(e.value()),
                                            option { value: "", "Select a dataset..." }
                                            for ds in list {
                                                option { value: "{ds.id}", "{ds.name} ({ds.dataset_type})" }
                                            }
                                        }
                                    }
                                }
                                Some(Err(_)) => rsx! {
                                    p { class: "text-[#ff3333] text-sm", "Failed to load datasets" }
                                },
                                None => rsx! {
                                    p { class: "text-[#888] text-sm animate-pulse", "Loading datasets..." }
                                },
                            }
                        }

                        // Error
                        if let Some(err) = error_msg() {
                            p { class: "text-[#ff3333] text-sm", "{err}" }
                        }

                        // Submit
                        button {
                            class: "bg-white text-black hover:bg-[#ddd] font-medium py-2 px-6 rounded-lg transition-colors disabled:opacity-50",
                            disabled: creating(),
                            onclick: handle_create,
                            if creating() {
                                "Creating..."
                            } else {
                                "Create"
                            }
                        }
                    }
                }
            }

            // Workspace list
            match workspaces() {
                Some(Ok(list)) => {
                    if list.is_empty() {
                        rsx! {
                            div { class: "text-center py-20",
                                div { class: "text-6xl mb-4 opacity-30", "🔬" }
                                p { class: "text-[#888] text-lg", "No workspaces yet" }
                                p { class: "text-[#555] mt-2", "Click \"+ New Workspace\" to get started" }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                                for ws in list {
                                    WorkspaceCard {
                                        key: "{ws.id}",
                                        workspace: ws.clone(),
                                        on_delete: move |_| {
                                            workspaces.restart();
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    div { class: "text-center py-20",
                        p { class: "text-[#ff3333] text-lg", "Error loading workspaces: {e}" }
                        button {
                            class: "mt-4 border border-[#444] text-white hover:bg-[#111] py-2 px-4 rounded-lg transition-colors",
                            onclick: move |_| workspaces.restart(),
                            "Retry"
                        }
                    }
                },
                None => rsx! {
                    div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                        for _ in 0..3 {
                            div { class: "border border-[#2a2a2a] rounded-xl p-6 animate-pulse",
                                div { class: "h-5 bg-[#1a1a1a] rounded w-3/4 mb-3" }
                                div { class: "h-4 bg-[#1a1a1a] rounded w-1/2 mb-2" }
                                div { class: "h-4 bg-[#1a1a1a] rounded w-1/3" }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn WorkspaceCard(workspace: crate::models::Workspace, on_delete: EventHandler) -> Element {
    let mut confirming = use_signal(|| false);
    let ws_id = workspace.id.clone();

    let created = workspace.created_at.split('T').next().unwrap_or(&workspace.created_at);
    let updated = workspace.updated_at.split('T').next().unwrap_or(&workspace.updated_at);

    rsx! {
        div { class: "card-surface rounded-xl p-6 group",
            // Header
            div { class: "flex items-start justify-between mb-3",
                Link {
                    to: Route::WorkspaceDetail {
                        id: workspace.id.clone(),
                    },
                    class: "text-lg font-semibold text-white hover:underline cursor-pointer",
                    "{workspace.name}"
                }
                if confirming() {
                    div { class: "flex gap-2",
                        button {
                            class: "text-xs text-[#ff3333] border border-[#ff3333] px-2 py-1 rounded hover:bg-[#ff3333] hover:text-black transition-colors",
                            onclick: {
                                let ws_id = ws_id.clone();
                                move |_| {
                                    let ws_id = ws_id.clone();
                                    async move {
                                        if delete_workspace_server(ws_id).await.is_ok() {
                                            on_delete.call(());
                                        }
                                    }
                                }
                            },
                            "Confirm"
                        }
                        button {
                            class: "text-xs text-[#888] border border-[#444] px-2 py-1 rounded hover:text-white transition-colors",
                            onclick: move |_| confirming.set(false),
                            "Cancel"
                        }
                    }
                } else {
                    button {
                        class: "text-[#555] hover:text-[#ff3333] transition-colors opacity-0 group-hover:opacity-100",
                        onclick: move |_| confirming.set(true),
                        "✕"
                    }
                }
            }

            // Dataset badge
            div { class: "mb-3",
                span { class: "text-xs border border-[#2a2a2a] text-[#888] px-2 py-1 rounded-full",
                    "📊 {workspace.dataset_name}"
                }
            }

            // Meta
            div { class: "flex justify-between text-xs text-[#555]",
                span { "Created {created}" }
                span { "Updated {updated}" }
            }
        }
    }
}
