use crate::Route;
use crate::models::Dataset;
use dioxus::prelude::*;

#[component]
pub fn DatasetCard(dataset: Dataset, on_delete: EventHandler) -> Element {
    let mut confirming = use_signal(|| false);
    let ds = dataset.clone();
    let ds_id = dataset.id.clone();

    rsx! {
        div { class: "card-surface rounded-xl p-6",
            // Header row
            div { class: "flex items-start justify-between mb-3",
                Link {
                    to: Route::DatasetDetail {
                        id: ds.id.clone(),
                    },
                    class: "text-lg font-semibold text-white hover:text-[#ccc] transition-colors",
                    "{ds.name}"
                }
                span { class: "px-2 py-0.5 rounded text-xs font-medium {ds.status_color()} ml-2 shrink-0",
                    "{ds.status}"
                }
            }

            // Type badge
            div { class: "mb-3",
                span { class: "px-2 py-1 border border-[#444] text-[#888] rounded text-xs font-medium",
                    "{ds.dataset_type}"
                }
                if !ds.format.is_empty() {
                    span { class: "px-2 py-1 border border-[#2a2a2a] text-[#555] rounded text-xs font-medium ml-2",
                        "{ds.format}"
                    }
                }
            }

            // Description preview
            if !ds.description.is_empty() {
                p { class: "text-[#888] text-sm mb-3 line-clamp-2", "{ds.description}" }
            }

            // Metadata row
            div { class: "flex items-center gap-4 text-xs text-[#555] mb-3",
                span { "📦 {ds.human_readable_size()}" }
                if let Some(n) = ds.num_samples {
                    span { "📊 {n} samples" }
                }
                if let Some(n) = ds.num_classes {
                    span { "🏷️ {n} classes" }
                }
            }

            // Tags
            if !ds.tags.is_empty() {
                div { class: "flex flex-wrap gap-1 mb-3",
                    for tag in ds.tags.iter().take(3) {
                        span { class: "px-2 py-0.5 border border-[#2a2a2a] text-[#555] rounded-full text-xs",
                            "{tag}"
                        }
                    }
                    if ds.tags.len() > 3 {
                        {
                            let extra = ds.tags.len() - 3;
                            rsx! {
                                span { class: "text-[#555] text-xs", "+{extra}" }
                            }
                        }
                    }
                }
            }

            // Date + delete
            div { class: "flex items-center justify-between pt-3 border-t border-[#2a2a2a]",
                {
                    let date = ds.created_at.chars().take(10).collect::<String>();
                    rsx! {
                        span { class: "text-xs text-[#555]", "{date}" }
                    }
                }
                if confirming() {
                    div { class: "flex items-center gap-2",
                        span { class: "text-xs text-[#888]", "Delete?" }
                        button {
                            class: "text-xs text-[#ff3333] hover:text-[#ff6666] font-medium",
                            onclick: move |_| {
                                let id = ds_id.clone();
                                let on_delete = on_delete;
                                async move {
                                    if let Ok(()) = crate::server::delete_dataset_server(id).await {
                                        on_delete.call(());
                                    }
                                }
                            },
                            "Yes"
                        }
                        button {
                            class: "text-xs text-[#555] hover:text-white",
                            onclick: move |_| confirming.set(false),
                            "No"
                        }
                    }
                } else {
                    button {
                        class: "text-xs text-[#555] hover:text-[#ff3333] transition-colors",
                        onclick: move |_| confirming.set(true),
                        "🗑 Delete"
                    }
                }
            }
        }
    }
}
