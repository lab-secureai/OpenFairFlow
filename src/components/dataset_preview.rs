use dioxus::prelude::*;
use crate::server::get_preview_server;

#[component]
pub fn DatasetPreviewComponent(dataset_id: String) -> Element {
    let id = dataset_id.clone();
    let preview = use_resource(move || {
        let id = id.clone();
        async move { get_preview_server(id).await }
    });

    rsx! {
        div { class: "border border-[#2a2a2a] rounded-xl p-6",
            h2 { class: "text-sm font-semibold text-[#888] uppercase tracking-wider mb-4",
                "Preview"
            }

            match preview() {
                Some(Ok(data)) => rsx! {
                    // Sample images
                    if !data.sample_images.is_empty() {
                        div { class: "mb-6",
                            h3 { class: "text-sm font-medium text-[#888] mb-3", "Sample Images" }
                            div { class: "grid grid-cols-4 md:grid-cols-6 gap-2",
                                for (i , img) in data.sample_images.iter().enumerate() {
                                    img {
                                        key: "{i}",
                                        class: "w-full aspect-square object-cover rounded-lg bg-black border border-[#2a2a2a]",
                                        src: "{img}",
                                        alt: "Sample {i}",
                                    }
                                }
                            }
                        }
                    }

                    // Class distribution
                    if !data.class_distribution.is_empty() {
                        div { class: "mb-6",
                            h3 { class: "text-sm font-medium text-[#888] mb-3", "Class Distribution" }
                            div { class: "space-y-2",
                                {
                                    let max_count = data
                                        .class_distribution
                                        .iter()
                                        .map(|(_, c)| *c)
                                        .max()
                                        .unwrap_or(1);
                                    rsx! {
                                        for (label , count) in data.class_distribution.iter() {
                                            div { class: "flex items-center gap-3",
                                                span { class: "text-xs text-[#888] w-20 text-right shrink-0", "{label}" }
                                                div { class: "flex-1 bg-black rounded-full h-4 overflow-hidden border border-[#2a2a2a]",
                                                    div {
                                                        class: "bg-white h-full rounded-full transition-all",
                                                        style: "width: {(*count as f64 / max_count as f64 * 100.0) as u32}%",
                                                    }
                                                }
                                                span { class: "text-xs text-[#555] w-12 shrink-0", "{count}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Summary stats table
                    if !data.summary.is_empty() {
                        div {
                            h3 { class: "text-sm font-medium text-[#888] mb-3", "Summary" }
                            div { class: "grid grid-cols-2 gap-2",
                                for (key , val) in data.summary.iter() {
                                    div { class: "flex justify-between py-1.5 px-3 bg-[#111] border border-[#2a2a2a] rounded",
                                        span { class: "text-sm text-[#888]", "{key}" }
                                        span { class: "text-sm text-white font-medium", "{val}" }
                                    }
                                }
                            }
                        }
                    }

                    // No preview data available
                    if data.sample_images.is_empty() && data.class_distribution.is_empty()
                        && data.summary.is_empty()
                    {
                        p { class: "text-[#555] text-center py-6", "No preview data available" }
                    }
                },
                Some(Err(e)) => rsx! {
                    p { class: "text-[#ff3333] text-sm", "Failed to load preview: {e}" }
                },
                None => rsx! {
                    div { class: "animate-pulse space-y-3",
                        div { class: "h-4 bg-[#1a1a1a] rounded w-1/4" }
                        div { class: "grid grid-cols-4 gap-2",
                            for _ in 0..8 {
                                div { class: "aspect-square bg-[#1a1a1a] rounded-lg" }
                            }
                        }
                    }
                },
            }
        }
    }
}
