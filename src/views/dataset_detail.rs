use dioxus::prelude::*;
use crate::server::{get_dataset_server, delete_dataset_server};
use crate::components::{DatasetPreviewComponent, DatasetViewer};
use crate::Route;

#[component]
pub fn DatasetDetail(id: String) -> Element {
    let id_clone = id.clone();
    let dataset = use_server_future(move || {
        let id = id_clone.clone();
        async move { get_dataset_server(id).await }
    })?;

    let nav = use_navigator();

    rsx! {
        div { class: "max-w-4xl mx-auto px-4 sm:px-6 py-6 sm:py-8",
            // Back link
            Link {
                to: Route::Datasets {},
                class: "text-[#888] hover:text-white mb-6 inline-block transition-colors text-sm",
                "← Back to Datasets"
            }

            match dataset() {
                Some(Ok(Some(ds))) => {
                    let ds_id = ds.id.clone();
                    let ds_id2 = ds.id.clone();
                    rsx! {
                        // Header
                        div { class: "flex flex-col sm:flex-row items-start justify-between gap-4 mb-8",
                            div {
                                h1 { class: "text-2xl sm:text-3xl font-bold text-white mb-2 tracking-tight",
                                    "{ds.name}"
                                }
                                div { class: "flex items-center gap-3 flex-wrap",
                                    span { class: "px-3 py-1 rounded text-xs font-medium border border-[#444] text-[#888]",
                                        "{ds.dataset_type}"
                                    }
                                    span { class: "px-3 py-1 rounded text-xs font-medium {ds.status_color()}",
                                        "{ds.status}"
                                    }
                                }
                            }
                            button {
                                class: "border border-[#ff3333] text-[#ff3333] hover:bg-[#ff3333] hover:text-black font-medium py-2 px-4 rounded-lg transition-colors shrink-0",
                                onclick: move |_| {
                                    let id = ds_id.clone();
                                    let nav = nav.clone();
                                    async move {
                                        if let Ok(()) = delete_dataset_server(id).await {
                                            nav.push(Route::Datasets {});
                                        }
                                    }
                                },
                                "Delete"
                            }
                        }

                        // Description
                        if !ds.description.is_empty() {
                            div { class: "border border-[#2a2a2a] rounded-xl p-4 sm:p-6 mb-6",
                                h2 { class: "text-sm font-semibold text-[#888] uppercase tracking-wider mb-2",
                                    "Description"
                                }
                                p { class: "text-[#ccc]", "{ds.description}" }
                            }
                        }

                        // Metadata grid
                        div { class: "border border-[#2a2a2a] rounded-xl p-4 sm:p-6 mb-6",
                            h2 { class: "text-sm font-semibold text-[#888] uppercase tracking-wider mb-4",
                                "Metadata"
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4",
                                MetadataItem { label: "Format", value: ds.format.clone() }
                                MetadataItem { label: "Size", value: ds.human_readable_size() }
                                MetadataItem {
                                    label: "Source",
                                    value: if ds.source == "local" { "Local upload".to_string() } else { ds.source.clone() },
                                }
                                MetadataItem {
                                    label: "Created",
                                    value: ds.created_at.chars().take(10).collect::<String>(),
                                }
                                if let Some(n) = ds.num_samples {
                                    MetadataItem { label: "Samples", value: n.to_string() }
                                }
                                if let Some(n) = ds.num_classes {
                                    MetadataItem { label: "Classes", value: n.to_string() }
                                }
                            }
                        }

                        // Tags
                        if !ds.tags.is_empty() {
                            div { class: "border border-[#2a2a2a] rounded-xl p-4 sm:p-6 mb-6",
                                h2 { class: "text-sm font-semibold text-[#888] uppercase tracking-wider mb-3",
                                    "Tags"
                                }
                                div { class: "flex flex-wrap gap-2",
                                    for tag in ds.tags.iter() {
                                        span { class: "px-3 py-1 border border-[#2a2a2a] text-[#888] rounded-full text-sm",
                                            "{tag}"
                                        }
                                    }
                                }
                            }
                        }

                        // Dataset Viewer (parquet-powered table)
                        DatasetViewer { dataset_id: ds_id2.clone() }

                        // Preview section
                        DatasetPreviewComponent { dataset_id: ds_id2 }
                    }
                }
                Some(Ok(None)) => rsx! {
                    div { class: "text-center py-20",
                        p { class: "text-[#888] text-lg", "Dataset not found" }
                        Link {
                            to: Route::Datasets {},
                            class: "mt-4 inline-block text-[#888] hover:text-white transition-colors",
                            "Back to Datasets"
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "text-center py-20",
                        p { class: "text-[#ff3333] text-lg", "Error: {e}" }
                    }
                },
                None => rsx! {
                    div { class: "animate-pulse",
                        div { class: "h-8 bg-[#1a1a1a] rounded w-1/3 mb-4" }
                        div { class: "h-4 bg-[#1a1a1a] rounded w-1/4 mb-8" }
                        div { class: "border border-[#2a2a2a] rounded-xl p-6 mb-6",
                            div { class: "h-4 bg-[#1a1a1a] rounded w-full mb-2" }
                            div { class: "h-4 bg-[#1a1a1a] rounded w-2/3" }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn MetadataItem(label: String, value: String) -> Element {
    rsx! {
        div { class: "min-w-0",
            p { class: "text-[#555] text-xs uppercase tracking-wider", "{label}" }
            p { class: "text-white font-medium truncate", "{value}" }
        }
    }
}
