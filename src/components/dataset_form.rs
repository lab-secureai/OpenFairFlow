use crate::server::{download_from_link_server, upload_dataset_server};
use dioxus::prelude::*;

#[component]
pub fn DatasetForm(on_success: EventHandler) -> Element {
    let mut active_tab = use_signal(|| "upload");
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut dataset_type = use_signal(|| "MNIST".to_string());
    let mut format = use_signal(|| "binary".to_string());
    let mut tags_input = use_signal(String::new);
    let mut num_samples = use_signal(String::new);
    let mut num_classes = use_signal(String::new);

    // Upload-specific
    let mut file_data = use_signal(Vec::<u8>::new);
    let mut file_name = use_signal(String::new);

    // Link-specific
    let mut url = use_signal(String::new);

    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(String::new);

    let handle_submit = move |_| async move {
        let name_val = name().trim().to_string();
        if name_val.is_empty() {
            error_msg.set("Name is required".to_string());
            return;
        }

        let desc = description().clone();
        let ds_type = dataset_type().clone();
        let fmt = format().clone();
        let tags: Vec<String> = tags_input()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let samples: Option<u64> = num_samples().parse().ok();
        let classes: Option<u32> = num_classes().parse().ok();
        let tab = active_tab();

        loading.set(true);
        error_msg.set(String::new());

        let result = if tab == "upload" {
            let data = file_data();
            let fname = file_name();
            if data.is_empty() {
                error_msg.set("Please select a file".to_string());
                loading.set(false);
                return;
            }
            upload_dataset_server(
                name_val, desc, ds_type, tags, fmt, samples, classes, data, fname,
            )
            .await
        } else {
            let url_val = url().trim().to_string();
            if url_val.is_empty() {
                error_msg.set("URL is required".to_string());
                loading.set(false);
                return;
            }
            download_from_link_server(
                name_val, desc, ds_type, tags, fmt, samples, classes, url_val,
            )
            .await
        };

        loading.set(false);
        match result {
            Ok(_) => on_success.call(()),
            Err(e) => error_msg.set(format!("{e}")),
        }
    };

    rsx! {
        div { class: "border border-[#2a2a2a] rounded-xl p-6 mb-8",
            h2 { class: "text-xl font-semibold text-white mb-4", "Add New Dataset" }

            // Tabs
            div { class: "flex gap-2 mb-6",
                button {
                    class: if active_tab() == "upload" { "px-4 py-2 rounded-lg text-sm font-medium bg-white text-black" } else { "px-4 py-2 rounded-lg text-sm font-medium border border-[#2a2a2a] text-[#888] hover:text-white hover:border-[#444]" },
                    onclick: move |_| active_tab.set("upload"),
                    "📁 Upload from Local"
                }
                button {
                    class: if active_tab() == "link" { "px-4 py-2 rounded-lg text-sm font-medium bg-white text-black" } else { "px-4 py-2 rounded-lg text-sm font-medium border border-[#2a2a2a] text-[#888] hover:text-white hover:border-[#444]" },
                    onclick: move |_| active_tab.set("link"),
                    "🔗 Download from Link"
                }
            }

            // Form fields
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4 mb-4",
                // Name
                div {
                    label { class: "block text-sm text-[#888] mb-1", "Name *" }
                    input {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        placeholder: "e.g. MNIST Training Set",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }

                // Dataset Type
                div {
                    label { class: "block text-sm text-[#888] mb-1", "Dataset Type" }
                    select {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        value: "{dataset_type}",
                        onchange: move |e| dataset_type.set(e.value()),
                        option { value: "MNIST", "MNIST" }
                        option { value: "CIFAR-10", "CIFAR-10" }
                        option { value: "CIFAR-100", "CIFAR-100" }
                        option { value: "Fashion-MNIST", "Fashion-MNIST" }
                        option { value: "SVHN", "SVHN" }
                        option { value: "ImageNet", "ImageNet" }
                        option { value: "Custom", "Custom" }
                    }
                }

                // Format
                div {
                    label { class: "block text-sm text-[#888] mb-1", "Format" }
                    select {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        value: "{format}",
                        onchange: move |e| format.set(e.value()),
                        option { value: "binary", "Binary" }
                        option { value: "csv", "CSV" }
                        option { value: "images", "Images" }
                        option { value: "compressed", "Compressed Archive" }
                        option { value: "other", "Other" }
                    }
                }

                // Tags
                div {
                    label { class: "block text-sm text-[#888] mb-1", "Tags (comma-separated)" }
                    input {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        placeholder: "e.g. classification, grayscale, handwritten",
                        value: "{tags_input}",
                        oninput: move |e| tags_input.set(e.value()),
                    }
                }

                // Num samples
                div {
                    label { class: "block text-sm text-[#888] mb-1", "Number of Samples" }
                    input {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        r#type: "number",
                        placeholder: "e.g. 60000",
                        value: "{num_samples}",
                        oninput: move |e| num_samples.set(e.value()),
                    }
                }

                // Num classes
                div {
                    label { class: "block text-sm text-[#888] mb-1", "Number of Classes" }
                    input {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        r#type: "number",
                        placeholder: "e.g. 10",
                        value: "{num_classes}",
                        oninput: move |e| num_classes.set(e.value()),
                    }
                }
            }

            // Description
            div { class: "mb-4",
                label { class: "block text-sm text-[#888] mb-1", "Description" }
                textarea {
                    class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none resize-y",
                    rows: "3",
                    placeholder: "Describe the dataset...",
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                }
            }

            // Tab-specific input
            if active_tab() == "upload" {
                div { class: "mb-4",
                    label { class: "block text-sm text-[#888] mb-1", "File *" }
                    div { class: "border-2 border-dashed border-[#2a2a2a] rounded-lg p-6 text-center hover:border-[#555] transition-colors",
                        input {
                            class: "hidden",
                            r#type: "file",
                            id: "file-input",
                            onchange: move |e: Event<FormData>| async move {
                                let files = e.files();
                                if let Some(file) = files.first() {
                                    file_name.set(file.name());
                                    if let Ok(bytes) = file.read_bytes().await {
                                        file_data.set(bytes.to_vec());
                                    }
                                }
                            },
                        }
                        label { r#for: "file-input", class: "cursor-pointer",
                            if file_name().is_empty() {
                                p { class: "text-[#555]", "📂 Click to select a file" }
                                p { class: "text-[#444] text-xs mt-1", "Supports any dataset format" }
                            } else {
                                p { class: "text-white", "📄 {file_name()}" }
                                {
                                    let size = file_data().len() as f64;
                                    let size_str = if size < 1024.0 {
                                        format!("{} B", file_data().len())
                                    } else if size < 1024.0 * 1024.0 {
                                        format!("{:.1} KB", size / 1024.0)
                                    } else {
                                        format!("{:.1} MB", size / (1024.0 * 1024.0))
                                    };
                                    rsx! {
                                        p { class: "text-[#555] text-xs mt-1", "{size_str}" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "mb-4",
                    label { class: "block text-sm text-[#888] mb-1", "Download URL *" }
                    input {
                        class: "w-full bg-black border border-[#2a2a2a] rounded-lg px-3 py-2 text-white text-sm focus:border-[#888] focus:outline-none",
                        placeholder: "https://example.com/dataset.tar.gz",
                        value: "{url}",
                        oninput: move |e| url.set(e.value()),
                    }
                }
            }

            // Error message
            if !error_msg().is_empty() {
                div { class: "mb-4 p-3 border border-[#ff3333] rounded-lg text-[#ff3333] text-sm",
                    "{error_msg}"
                }
            }

            // Submit
            button {
                class: "w-full bg-white hover:bg-[#ddd] disabled:bg-[#333] disabled:text-[#555] disabled:cursor-not-allowed text-black font-medium py-2.5 px-4 rounded-lg transition-colors",
                disabled: loading(),
                onclick: handle_submit,
                if loading() {
                    "Processing..."
                } else if active_tab() == "upload" {
                    "Upload Dataset"
                } else {
                    "Download Dataset"
                }
            }
        }
    }
}
