use dioxus::prelude::*;
use crate::server::get_dataset_viewer_server;
use crate::models::DatasetCell;

const ROWS_PER_PAGE: u64 = 10;

#[component]
pub fn DatasetViewer(dataset_id: String) -> Element {
    let mut current_split = use_signal(|| String::new());
    let mut current_offset = use_signal(|| 0u64);

    let id = dataset_id.clone();
    let viewer = use_resource(move || {
        let id = id.clone();
        let split = current_split();
        let offset = current_offset();
        async move { get_dataset_viewer_server(id, split, offset, ROWS_PER_PAGE).await }
    });

    rsx! {
        div { class: "border border-[#2a2a2a] rounded-xl overflow-hidden mb-6",
            // Header
            div { class: "px-4 sm:px-6 py-4 border-b border-[#2a2a2a] flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3",
                h2 { class: "text-sm font-semibold text-[#888] uppercase tracking-wider",
                    "Dataset Viewer"
                }
            }

            match viewer() {
                Some(Ok(page)) => {
                    // Sync the current_split if server picked a default
                    let page_split = page.split.clone();
                    let page_splits = page.available_splits.clone();

                    rsx! {
                        // Split tabs
                        if page_splits.len() > 1 {
                            div { class: "px-4 sm:px-6 py-3 border-b border-[#2a2a2a] flex gap-2 flex-wrap",
                                for s in page_splits.iter() {
                                    {
                                        let s_clone = s.clone();
                                        let is_active = *s == page_split;
                                        rsx! {
                                            button {
                                                class: if is_active { "px-3 py-1.5 rounded-lg text-sm font-medium bg-white text-black" } else { "px-3 py-1.5 rounded-lg text-sm font-medium text-[#888] hover:text-white border border-[#2a2a2a] hover:border-[#444] transition-colors" },
                                                onclick: move |_| {
                                                    current_split.set(s_clone.clone());
                                                    current_offset.set(0);
                                                },
                                                "{s}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Table
                        div { class: "overflow-x-auto",
                            table { class: "w-full text-sm",
                                thead {
                                    tr { class: "border-b border-[#2a2a2a]",
                                        th { class: "px-3 py-2.5 text-left text-xs font-medium text-[#555] uppercase tracking-wider w-12",
                                            "#"
                                        }
                                        for col in page.columns.iter() {
                                            th { class: "px-3 py-2.5 text-left text-xs font-medium text-[#555] uppercase tracking-wider",
                                                "{col.name}"
                                            }
                                        }
                                    }
                                }
                                tbody {
                                    for row in page.rows.iter() {
                                        tr { class: "border-b border-[#1a1a1a] hover:bg-[#111] transition-colors",
                                            td { class: "px-3 py-2 text-[#444] font-mono text-xs", "{row.index}" }
                                            for (_i , cell) in row.cells.iter().enumerate() {
                                                td { class: "px-3 py-2",
                                                    match cell {
                                                        DatasetCell::Image(src) => rsx! {
                                                            img {
                                                                class: "w-16 h-16 object-cover rounded bg-black border border-[#2a2a2a]",
                                                                src: "{src}",
                                                                alt: "Row {row.index}",
                                                            }
                                                        },
                                                        DatasetCell::Number(val) => rsx! {
                                                            span { class: "text-white font-mono", "{val}" }
                                                        },
                                                        DatasetCell::Text(val) => rsx! {
                                                            span { class: "text-[#ccc] max-w-xs truncate block", "{val}" }
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if page.rows.is_empty() {
                                div { class: "text-center py-8",
                                    p { class: "text-[#555]", "No rows in this split" }
                                }
                            }
                        }

                        // Pagination
                        div { class: "px-4 sm:px-6 py-3 border-t border-[#2a2a2a] flex items-center justify-between",
                            div { class: "flex items-center gap-2",
                                button {
                                    class: "px-3 py-1.5 rounded text-sm text-[#888] hover:text-white border border-[#2a2a2a] hover:border-[#444] transition-colors disabled:opacity-30 disabled:cursor-not-allowed",
                                    disabled: page.offset == 0,
                                    onclick: move |_| {
                                        let new_offset = current_offset().saturating_sub(ROWS_PER_PAGE);
                                        current_offset.set(new_offset);
                                    },
                                    "← Previous"
                                }
                                button {
                                    class: "px-3 py-1.5 rounded text-sm text-[#888] hover:text-white border border-[#2a2a2a] hover:border-[#444] transition-colors disabled:opacity-30 disabled:cursor-not-allowed",
                                    disabled: page.offset + page.limit >= page.total_rows,
                                    onclick: move |_| {
                                        current_offset.set(current_offset() + ROWS_PER_PAGE);
                                    },
                                    "Next →"
                                }
                            }
                            span { class: "text-xs text-[#555]",
                                {
                                    let start = page.offset + 1;
                                    let end = (page.offset + page.rows.len() as u64).min(page.total_rows);
                                    format!("Showing {start}\u{2013}{end} of {}", page.total_rows)
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    let err = format!("{e}");
                    rsx! {
                        div { class: "px-6 py-8 text-center",
                            p { class: "text-[#555] text-sm", "No viewer data available" }
                            p { class: "text-[#333] text-xs mt-1", "{err}" }
                        }
                    }
                }
                None => rsx! {
                    div { class: "p-6 animate-pulse space-y-3",
                        div { class: "flex gap-2 mb-4",
                            div { class: "h-8 bg-[#1a1a1a] rounded w-16" }
                            div { class: "h-8 bg-[#1a1a1a] rounded w-16" }
                        }
                        for _ in 0..5 {
                            div { class: "flex gap-3",
                                div { class: "h-4 bg-[#1a1a1a] rounded w-8" }
                                div { class: "h-12 w-12 bg-[#1a1a1a] rounded" }
                                div { class: "h-4 bg-[#1a1a1a] rounded w-12" }
                            }
                        }
                    }
                },
            }
        }
    }
}
