use crate::Route;
use crate::server::{list_datasets_server, list_workspaces_server};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let datasets = use_server_future(move || list_datasets_server())?;
    let workspaces = use_server_future(move || list_workspaces_server())?;

    let (total, ready, total_size, recent) = match datasets() {
        Some(Ok(ref list)) => {
            let total = list.len();
            let ready = list.iter().filter(|d| d.status == "ready").count();
            let total_size: u64 = list.iter().map(|d| d.file_size).sum();
            let recent: Vec<_> = list.iter().rev().take(5).cloned().collect();
            (total, ready, total_size, recent)
        }
        _ => (0, 0, 0u64, vec![]),
    };

    let (ws_count, recent_ws) = match workspaces() {
        Some(Ok(ref list)) => {
            let mut sorted = list.clone();
            sorted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            let recent: Vec<_> = sorted.into_iter().take(5).collect();
            (list.len(), recent)
        }
        _ => (0, vec![]),
    };

    let size_str = if total_size < 1024 {
        format!("{total_size} B")
    } else if total_size < 1024 * 1024 {
        format!("{:.1} KB", total_size as f64 / 1024.0)
    } else if total_size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", total_size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", total_size as f64 / (1024.0 * 1024.0 * 1024.0))
    };

    rsx! {
        div { class: "max-w-5xl mx-auto px-4 py-12",
            // Title
            div { class: "mb-10",
                h1 { class: "text-4xl font-bold tracking-tight text-white mb-2", "Dashboard" }
                p { class: "text-[#888] text-lg", "Federated learning platform" }
            }

            // Stats cards
            div { class: "grid grid-cols-1 sm:grid-cols-4 gap-4 mb-10",
                StatCard { label: "Total Datasets", value: total.to_string() }
                StatCard { label: "Ready", value: ready.to_string() }
                StatCard { label: "Total Size", value: size_str }
                StatCard { label: "Workspaces", value: ws_count.to_string() }
            }

            // Recent datasets
            div { class: "border border-[#2a2a2a] rounded-xl overflow-hidden",
                div { class: "px-6 py-4 border-b border-[#2a2a2a] flex items-center justify-between",
                    h2 { class: "text-sm font-semibold text-white uppercase tracking-wider",
                        "Recent Datasets"
                    }
                    Link {
                        to: Route::Datasets {},
                        class: "text-xs text-[#888] hover:text-white transition-colors",
                        "View all →"
                    }
                }
                if recent.is_empty() {
                    div { class: "px-6 py-12 text-center",
                        p { class: "text-[#555]", "No datasets yet." }
                        Link {
                            to: Route::Datasets {},
                            class: "inline-block mt-3 text-sm text-white border border-[#444] px-4 py-2 rounded-lg hover:bg-[#111] transition-colors",
                            "+ Add Dataset"
                        }
                    }
                } else {
                    for ds in recent {
                        Link {
                            to: Route::DatasetDetail {
                                id: ds.id.clone(),
                            },
                            class: "flex items-center justify-between px-6 py-3 border-b border-[#1a1a1a] hover:bg-[#111] transition-colors",
                            div { class: "flex items-center gap-3",
                                span { class: "text-sm font-medium text-white", "{ds.name}" }
                                span { class: "text-xs text-[#555] border border-[#2a2a2a] px-2 py-0.5 rounded",
                                    "{ds.dataset_type}"
                                }
                            }
                            div { class: "flex items-center gap-4 text-xs text-[#555]",
                                span { "{ds.human_readable_size()}" }
                                span { "{ds.status}" }
                            }
                        }
                    }
                }
            }

            // Recent workspaces
            div { class: "border border-[#2a2a2a] rounded-xl overflow-hidden mt-6",
                div { class: "px-6 py-4 border-b border-[#2a2a2a] flex items-center justify-between",
                    h2 { class: "text-sm font-semibold text-white uppercase tracking-wider",
                        "Recent Workspaces"
                    }
                    Link {
                        to: Route::Workspaces {},
                        class: "text-xs text-[#888] hover:text-white transition-colors",
                        "View all →"
                    }
                }
                if recent_ws.is_empty() {
                    div { class: "px-6 py-12 text-center",
                        p { class: "text-[#555]", "No workspaces yet." }
                        Link {
                            to: Route::Workspaces {},
                            class: "inline-block mt-3 text-sm text-white border border-[#444] px-4 py-2 rounded-lg hover:bg-[#111] transition-colors",
                            "+ New Workspace"
                        }
                    }
                } else {
                    for ws in recent_ws {
                        Link {
                            to: Route::WorkspaceDetail {
                                id: ws.id.clone(),
                            },
                            class: "flex items-center justify-between px-6 py-3 border-b border-[#1a1a1a] hover:bg-[#111] transition-colors",
                            div { class: "flex items-center gap-3",
                                span { class: "text-sm font-medium text-white", "{ws.name}" }
                                span { class: "text-xs text-[#555] border border-[#2a2a2a] px-2 py-0.5 rounded",
                                    "{ws.dataset_name}"
                                }
                            }
                            span { class: "text-xs text-[#555]",
                                "{ws.updated_at.split('T').next().unwrap_or(&ws.updated_at)}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String) -> Element {
    rsx! {
        div { class: "border border-[#2a2a2a] rounded-xl px-6 py-5",
            p { class: "text-xs text-[#555] uppercase tracking-wider mb-1", "{label}" }
            p { class: "text-2xl font-bold text-white", "{value}" }
        }
    }
}
