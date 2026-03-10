use dioxus::prelude::*;
use crate::server::list_datasets_server;
use crate::components::{DatasetCard, DatasetForm};

const DATASETS_CSS: Asset = asset!("/assets/styling/datasets.css");

#[component]
pub fn Datasets() -> Element {
    let mut show_form = use_signal(|| false);
    let mut datasets = use_server_future(move || list_datasets_server())?;

    rsx! {
        document::Link { rel: "stylesheet", href: DATASETS_CSS }

        div { class: "max-w-7xl mx-auto px-4 py-8",
            // Header
            div { class: "flex items-center justify-between mb-8",
                h1 { class: "text-3xl font-bold tracking-tight text-white", "Datasets" }
                button {
                    class: if show_form() { "border border-[#444] text-[#888] hover:text-white hover:border-[#666] font-medium py-2 px-4 rounded-lg transition-colors" } else { "bg-white text-black hover:bg-[#ddd] font-medium py-2 px-4 rounded-lg transition-colors" },
                    onclick: move |_| show_form.set(!show_form()),
                    if show_form() {
                        "Cancel"
                    } else {
                        "+ Add Dataset"
                    }
                }
            }

            // Add Dataset Form
            if show_form() {
                DatasetForm {
                    on_success: move |_| {
                        show_form.set(false);
                        datasets.restart();
                    },
                }
            }

            // Dataset Grid
            match datasets() {
                Some(Ok(list)) => {
                    if list.is_empty() {
                        rsx! {
                            div { class: "text-center py-20",
                                div { class: "text-6xl mb-4 opacity-30", "📂" }
                                p { class: "text-[#888] text-lg", "No datasets yet" }
                                p { class: "text-[#555] mt-2", "Click \"+ Add Dataset\" to get started" }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                                for ds in list {
                                    DatasetCard {
                                        key: "{ds.id}",
                                        dataset: ds.clone(),
                                        on_delete: move |_| {
                                            datasets.restart();
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    div { class: "text-center py-20",
                        p { class: "text-[#ff3333] text-lg", "Error loading datasets: {e}" }
                        button {
                            class: "mt-4 border border-[#444] text-white hover:bg-[#111] py-2 px-4 rounded-lg transition-colors",
                            onclick: move |_| datasets.restart(),
                            "Retry"
                        }
                    }
                },
                None => rsx! {
                    div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                        for _ in 0..6 {
                            div { class: "border border-[#2a2a2a] rounded-xl p-6 animate-pulse",
                                div { class: "h-4 bg-[#1a1a1a] rounded w-3/4 mb-4" }
                                div { class: "h-3 bg-[#1a1a1a] rounded w-1/2 mb-2" }
                                div { class: "h-3 bg-[#1a1a1a] rounded w-1/3" }
                            }
                        }
                    }
                },
            }
        }
    }
}
