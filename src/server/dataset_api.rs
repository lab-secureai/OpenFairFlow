use dioxus::prelude::*;
use crate::models::{Dataset, DatasetPreview, DatasetViewerPage};
#[cfg(feature = "server")]
use crate::models::{DatasetViewerRow, DatasetCell, ColumnInfo, ColumnType};

#[post("/api/datasets/list")]
pub async fn list_datasets_server() -> Result<Vec<Dataset>, ServerFnError> {
    crate::db::list_datasets().map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[post("/api/datasets/get")]
pub async fn get_dataset_server(id: String) -> Result<Option<Dataset>, ServerFnError> {
    crate::db::get_dataset(&id).map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[post("/api/datasets/upload")]
pub async fn upload_dataset_server(
    name: String,
    description: String,
    dataset_type: String,
    tags: Vec<String>,
    format: String,
    num_samples: Option<u64>,
    num_classes: Option<u32>,
    file_data: Vec<u8>,
    file_name: String,
) -> Result<Dataset, ServerFnError> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = format!("data/datasets/{id}");
    tokio::fs::create_dir_all(&dir).await.map_err(|e| ServerFnError::new(format!("IO error: {e}")))?;

    let file_path = format!("{dir}/{file_name}");
    let file_size = file_data.len() as u64;
    tokio::fs::write(&file_path, &file_data).await.map_err(|e| ServerFnError::new(format!("IO error: {e}")))?;

    let dataset = Dataset {
        id,
        name,
        dataset_type,
        description,
        tags,
        format,
        num_samples,
        num_classes,
        file_size,
        source: "local".to_string(),
        file_path,
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "ready".to_string(),
    };

    crate::db::insert_dataset(&dataset).map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;
    Ok(dataset)
}

#[post("/api/datasets/download_link")]
pub async fn download_from_link_server(
    name: String,
    description: String,
    dataset_type: String,
    tags: Vec<String>,
    format: String,
    num_samples: Option<u64>,
    num_classes: Option<u32>,
    url: String,
) -> Result<Dataset, ServerFnError> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = format!("data/datasets/{id}");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| ServerFnError::new(format!("IO error: {e}")))?;

    let is_hf = is_huggingface_url(&url);

    let file_name = if is_hf {
        "huggingface_dataset".to_string()
    } else {
        url.split('/')
            .last()
            .filter(|s| !s.is_empty())
            .unwrap_or("dataset")
            .to_string()
    };
    let file_path = format!("{dir}/{file_name}");

    // Insert record with downloading status
    let dataset = Dataset {
        id: id.clone(),
        name,
        dataset_type,
        description,
        tags,
        format,
        num_samples,
        num_classes,
        file_size: 0,
        source: url.clone(),
        file_path: file_path.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "downloading".to_string(),
    };
    crate::db::insert_dataset(&dataset)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

    let result = if is_hf {
        download_huggingface_dataset(&url, &dir).await
    } else {
        download_direct_file(&url, &file_path).await
    };

    match result {
        Ok(file_size) => {
            crate::db::update_dataset_status(&id, "ready", Some(file_size))
                .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;
        }
        Err(e) => {
            let _ = crate::db::update_dataset_status(&id, "error", None);
            return Err(e);
        }
    }

    crate::db::get_dataset(&id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Dataset not found after download"))
}

#[cfg(feature = "server")]
fn is_huggingface_url(url: &str) -> bool {
    url.contains("huggingface.co/datasets/")
}

/// Parse repo ID (e.g. "ylecun/mnist") from a HuggingFace dataset URL.
#[cfg(feature = "server")]
fn parse_hf_repo_id(url: &str) -> Option<String> {
    let marker = "huggingface.co/datasets/";
    let idx = url.find(marker)?;
    let after = &url[idx + marker.len()..];
    let clean = after.trim_end_matches('/');
    let parts: Vec<&str> = clean.split('/').collect();
    match parts.len() {
        0 => None,
        1 => Some(parts[0].to_string()),
        _ => Some(format!("{}/{}", parts[0], parts[1])),
    }
}

#[cfg(feature = "server")]
async fn download_direct_file(url: &str, file_path: &str) -> Result<u64, ServerFnError> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| ServerFnError::new(format!("Download error: {e}")))?;
    if !response.status().is_success() {
        return Err(ServerFnError::new(format!("HTTP {}", response.status())));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| ServerFnError::new(format!("Download error: {e}")))?;
    tokio::fs::write(file_path, &bytes)
        .await
        .map_err(|e| ServerFnError::new(format!("IO error: {e}")))?;
    Ok(bytes.len() as u64)
}

/// Download all files from a HuggingFace dataset repo via the Hub API.
#[cfg(feature = "server")]
async fn download_huggingface_dataset(url: &str, dir: &str) -> Result<u64, ServerFnError> {
    let repo_id = parse_hf_repo_id(url)
        .ok_or_else(|| ServerFnError::new("Could not parse HuggingFace repo ID from URL"))?;

    let client = reqwest::Client::new();

    // List all files via HF tree API
    let tree_url = format!(
        "https://huggingface.co/api/datasets/{repo_id}/tree/main?recursive=true"
    );
    let tree_resp = client
        .get(&tree_url)
        .header("User-Agent", "fed-lab/0.1")
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("HF API error: {e}")))?;

    if !tree_resp.status().is_success() {
        let status = tree_resp.status();
        let body = tree_resp.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!(
            "HF API returned {status}. Is \"{repo_id}\" a valid public dataset? {body}"
        )));
    }

    let tree_body = tree_resp
        .text()
        .await
        .map_err(|e| ServerFnError::new(format!("HF API error: {e}")))?;
    let entries: Vec<HfTreeEntry> = serde_json::from_str(&tree_body)
        .map_err(|e| ServerFnError::new(format!("HF API parse error: {e}")))?;

    let files: Vec<&HfTreeEntry> = entries.iter().filter(|e| e.entry_type == "file").collect();
    if files.is_empty() {
        return Err(ServerFnError::new(
            "No files found in this HuggingFace dataset",
        ));
    }

    let mut total_size: u64 = 0;

    for file_entry in &files {
        let resolve_url = format!(
            "https://huggingface.co/datasets/{repo_id}/resolve/main/{}",
            file_entry.path
        );

        let local_path = format!("{dir}/{}", file_entry.path);
        if let Some(parent) = std::path::Path::new(&local_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ServerFnError::new(format!("IO error: {e}")))?;
        }

        let resp = client
            .get(&resolve_url)
            .header("User-Agent", "fed-lab/0.1")
            .send()
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Download error for {}: {e}", file_entry.path))
            })?;

        if !resp.status().is_success() {
            continue; // skip gated/inaccessible files
        }

        let bytes = resp.bytes().await.map_err(|e| {
            ServerFnError::new(format!("Download error for {}: {e}", file_entry.path))
        })?;

        tokio::fs::write(&local_path, &bytes)
            .await
            .map_err(|e| ServerFnError::new(format!("IO error: {e}")))?;

        total_size += bytes.len() as u64;
    }

    if total_size == 0 {
        return Err(ServerFnError::new(
            "Could not download any files (dataset may require authentication)",
        ));
    }

    Ok(total_size)
}

#[cfg(feature = "server")]
#[derive(serde::Deserialize)]
struct HfTreeEntry {
    #[serde(rename = "type")]
    entry_type: String,
    path: String,
}

#[post("/api/datasets/delete")]
pub async fn delete_dataset_server(id: String) -> Result<(), ServerFnError> {
    // Get dataset to find file path
    let dataset = crate::db::get_dataset(&id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

    if let Some(ds) = dataset {
        // Remove the dataset directory
        let dir = format!("data/datasets/{}", ds.id);
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    crate::db::delete_dataset_db(&id).map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;
    Ok(())
}

#[post("/api/datasets/preview")]
pub async fn get_preview_server(id: String) -> Result<DatasetPreview, ServerFnError> {
    let dataset = crate::db::get_dataset(&id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Dataset not found"))?;

    let mut preview = DatasetPreview {
        sample_images: Vec::new(),
        class_distribution: Vec::new(),
        summary: Vec::new(),
    };

    // Basic summary from metadata
    preview.summary.push(("Name".to_string(), dataset.name.clone()));
    preview.summary.push(("Type".to_string(), dataset.dataset_type.clone()));
    preview.summary.push(("Format".to_string(), dataset.format.clone()));
    preview.summary.push(("Size".to_string(), dataset.human_readable_size()));
    preview.summary.push(("Source".to_string(), dataset.source.clone()));
    preview.summary.push(("Status".to_string(), dataset.status.clone()));

    if let Some(n) = dataset.num_samples {
        preview.summary.push(("Samples".to_string(), n.to_string()));
    }
    if let Some(n) = dataset.num_classes {
        preview.summary.push(("Classes".to_string(), n.to_string()));
    }

    // Try to generate image thumbnails from the dataset directory
    let dir = std::path::Path::new(&dataset.file_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    if dir.exists() {
        let mut image_paths: Vec<std::path::PathBuf> = Vec::new();
        collect_images(dir, &mut image_paths, 12);

        for path in image_paths.iter().take(12) {
            if let Ok(img) = image::open(path) {
                let thumb = img.thumbnail(64, 64);
                let mut buf = std::io::Cursor::new(Vec::new());
                if thumb.write_to(&mut buf, image::ImageFormat::Png).is_ok() {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
                    preview.sample_images.push(format!("data:image/png;base64,{b64}"));
                }
            }
        }
    }

    // If no loose images found, try reading from parquet files
    if preview.sample_images.is_empty() {
        let base_dir = format!("data/datasets/{}", dataset.id);
        let base_path = std::path::Path::new(&base_dir);
        if base_path.exists() {
            let splits = scan_parquet_splits(base_path);
            // Use the first split (prefer "train")
            let parquet_path = splits.iter()
                .find(|(s, _)| s == "train")
                .or_else(|| splits.first())
                .map(|(_, p)| p.clone());

            if let Some(path) = parquet_path {
                let _ = read_parquet_preview(&path, &mut preview);
            }
        }
    }

    Ok(preview)
}

#[cfg(feature = "server")]
fn collect_images(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, max: usize) {
    if out.len() >= max {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if out.len() >= max {
                return;
            }
            let path = entry.path();
            if path.is_dir() {
                collect_images(&path, out, max);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" => {
                        out.push(path);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Extract sample images and class distribution from a parquet file for the preview component
#[cfg(feature = "server")]
fn read_parquet_preview(path: &std::path::Path, preview: &mut DatasetPreview) -> Result<(), Box<dyn std::error::Error>> {
    use parquet::file::reader::SerializedFileReader;
    use parquet::file::reader::FileReader;
    use parquet::record::Field;

    let file = std::fs::File::open(path)?;
    let reader = SerializedFileReader::new(file)?;

    // Detect image and label columns from schema
    let schema = reader.metadata().file_metadata().schema_descr();
    let schema_root = schema.root_schema();
    let mut image_col: Option<String> = None;
    let mut label_col: Option<String> = None;

    if let parquet::schema::types::Type::GroupType { fields, .. } = schema_root {
        for field in fields {
            let name = field.name().to_string();
            if let parquet::schema::types::Type::GroupType { fields: sub, .. } = field.as_ref() {
                if sub.iter().any(|f| f.name() == "bytes") && image_col.is_none() {
                    image_col = Some(name.clone());
                }
            }
            if name == "label" || name == "labels" {
                label_col = Some(name.clone());
            }
        }
    }

    let mut row_iter = reader.get_row_iter(None)?;
    let mut label_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let max_samples = 12;
    let max_label_scan = 5000; // Scan up to 5000 rows for class distribution

    for _ in 0..max_label_scan {
        let Some(row_result) = row_iter.next() else { break };
        let row = row_result?;

        // Extract sample images from first 12 rows
        if preview.sample_images.len() < max_samples {
            if let Some(ref img_name) = image_col {
                if let Some((_, field)) = row.get_column_iter().find(|(n, _)| *n == img_name) {
                    if let Field::Group(group) = field {
                        for (n, sub) in group.get_column_iter() {
                            if n == "bytes" {
                                if let Field::Bytes(bytes) = sub {
                                    if let Ok(img) = image::load_from_memory(bytes.data()) {
                                        let thumb = img.thumbnail(64, 64);
                                        let mut buf = std::io::Cursor::new(Vec::new());
                                        if thumb.write_to(&mut buf, image::ImageFormat::Png).is_ok() {
                                            use base64::Engine;
                                            let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
                                            preview.sample_images.push(format!("data:image/png;base64,{b64}"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Count labels for class distribution
        if let Some(ref lbl_name) = label_col {
            if let Some((_, field)) = row.get_column_iter().find(|(n, _)| *n == lbl_name) {
                let label_str = match field {
                    Field::Int(v) => v.to_string(),
                    Field::Long(v) => v.to_string(),
                    Field::Str(s) => s.clone(),
                    other => format!("{other}"),
                };
                *label_counts.entry(label_str).or_insert(0) += 1;
            }
        }
    }

    // Sort class distribution by label
    if !label_counts.is_empty() {
        let mut sorted: Vec<(String, u32)> = label_counts.into_iter().collect();
        sorted.sort_by(|a, b| {
            // Try numeric sort first, fall back to string sort
            a.0.parse::<i64>().ok().cmp(&b.0.parse::<i64>().ok())
                .then_with(|| a.0.cmp(&b.0))
        });
        preview.class_distribution = sorted;
    }

    Ok(())
}

// ── Dataset Viewer (Parquet-powered) ──────────────────────────────────────

#[post("/api/datasets/viewer")]
pub async fn get_dataset_viewer_server(
    id: String,
    split: String,
    offset: u64,
    limit: u64,
) -> Result<DatasetViewerPage, ServerFnError> {
    let dataset = crate::db::get_dataset(&id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Dataset not found"))?;

    let base_dir = format!("data/datasets/{}", dataset.id);
    let base_path = std::path::Path::new(&base_dir);

    if !base_path.exists() {
        return Err(ServerFnError::new("Dataset directory not found"));
    }

    let splits = scan_parquet_splits(base_path);
    if splits.is_empty() {
        return Err(ServerFnError::new("No parquet files found in dataset"));
    }

    let available_splits: Vec<String> = splits.iter().map(|(s, _)| s.clone()).collect();
    let active_split = if available_splits.contains(&split) {
        split
    } else {
        available_splits[0].clone()
    };

    let parquet_path = splits
        .iter()
        .find(|(s, _)| *s == active_split)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| ServerFnError::new("Split not found"))?;

    read_parquet_page(&parquet_path, &active_split, &available_splits, offset, limit)
}

#[cfg(feature = "server")]
fn scan_parquet_splits(dir: &std::path::Path) -> Vec<(String, std::path::PathBuf)> {
    let mut splits: std::collections::BTreeMap<String, std::path::PathBuf> = std::collections::BTreeMap::new();
    collect_parquet_files(dir, &mut splits);
    splits.into_iter().collect()
}

#[cfg(feature = "server")]
fn collect_parquet_files(dir: &std::path::Path, out: &mut std::collections::BTreeMap<String, std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_parquet_files(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("parquet") {
                let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                // Parse split name from pattern: {split}-{shard}-of-{total}
                let split_name = if let Some(idx) = file_stem.find('-') {
                    file_stem[..idx].to_string()
                } else {
                    file_stem.to_string()
                };
                out.entry(split_name).or_insert(path);
            }
        }
    }
}

#[cfg(feature = "server")]
fn read_parquet_page(
    path: &std::path::Path,
    split: &str,
    available_splits: &[String],
    offset: u64,
    limit: u64,
) -> Result<DatasetViewerPage, ServerFnError> {
    use parquet::file::reader::SerializedFileReader;
    use parquet::file::reader::FileReader;
    use std::fs::File;

    let file = File::open(path)
        .map_err(|e| ServerFnError::new(format!("Cannot open parquet file: {e}")))?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| ServerFnError::new(format!("Parquet read error: {e}")))?;

    let metadata = reader.metadata();
    let total_rows: u64 = metadata.file_metadata().num_rows() as u64;
    let schema = metadata.file_metadata().schema_descr();

    // Build column info from schema
    let columns: Vec<ColumnInfo> = schema
        .columns()
        .iter()
        .filter(|col| {
            // Skip nested struct sub-fields (only show top-level columns)
            col.path().parts().len() == 1
        })
        .map(|col| {
            let name = col.name().to_string();
            let col_type = match col.physical_type() {
                parquet::basic::Type::INT32
                | parquet::basic::Type::INT64
                | parquet::basic::Type::INT96
                | parquet::basic::Type::FLOAT
                | parquet::basic::Type::DOUBLE => ColumnType::Number,
                parquet::basic::Type::BYTE_ARRAY | parquet::basic::Type::FIXED_LEN_BYTE_ARRAY => {
                    // Check if this is part of an image struct by looking at the schema
                    // HF stores images as struct { bytes: BYTE_ARRAY, path: BYTE_ARRAY }
                    // The top-level "image" or "img" column is a group, its "bytes" sub-field
                    // won't appear here because we filter to path len == 1
                    ColumnType::Text
                }
                _ => ColumnType::Text,
            };
            ColumnInfo { name, col_type }
        })
        .collect();

    // Detect image columns by checking for group columns in the schema
    // HF datasets store images as group { bytes, path } — the top-level group name is "image" or "img"
    let schema_root = schema.root_schema();
    let mut image_col_names: Vec<String> = Vec::new();
    let mut top_level_names: Vec<String> = Vec::new();

    if let parquet::schema::types::Type::GroupType { fields, .. } = schema_root {
        for field in fields {
            let field_name = field.name().to_string();
            top_level_names.push(field_name.clone());
            if let parquet::schema::types::Type::GroupType { fields: sub_fields, .. } = field.as_ref() {
                let has_bytes = sub_fields.iter().any(|f| f.name() == "bytes");
                if has_bytes {
                    image_col_names.push(field_name);
                }
            }
        }
    }

    // Rebuild columns with correct types for image columns
    let columns: Vec<ColumnInfo> = top_level_names
        .iter()
        .map(|name| {
            if image_col_names.contains(name) {
                ColumnInfo { name: name.clone(), col_type: ColumnType::Image }
            } else {
                // Find from the previously detected columns
                columns
                    .iter()
                    .find(|c| &c.name == name)
                    .cloned()
                    .unwrap_or(ColumnInfo { name: name.clone(), col_type: ColumnType::Text })
            }
        })
        .collect();

    // Read rows using row iter
    let mut row_iter = reader.get_row_iter(None)
        .map_err(|e| ServerFnError::new(format!("Parquet row iter error: {e}")))?;

    // Skip to offset
    for _ in 0..offset {
        if row_iter.next().is_none() {
            break;
        }
    }

    let mut rows: Vec<DatasetViewerRow> = Vec::new();
    for i in 0..limit {
        let Some(row_result) = row_iter.next() else { break };
        let row = row_result.map_err(|e| ServerFnError::new(format!("Row read error: {e}")))?;

        let mut cells: Vec<DatasetCell> = Vec::new();
        for col_info in &columns {
            let cell = match row.get_column_iter().find(|(name, _)| *name == &col_info.name) {
                Some((_, field)) => field_to_cell(field, &col_info.col_type),
                None => DatasetCell::Text("—".to_string()),
            };
            cells.push(cell);
        }

        rows.push(DatasetViewerRow {
            index: offset + i,
            cells,
        });
    }

    Ok(DatasetViewerPage {
        columns,
        rows,
        total_rows,
        offset,
        limit,
        split: split.to_string(),
        available_splits: available_splits.to_vec(),
    })
}

#[cfg(feature = "server")]
fn field_to_cell(field: &parquet::record::Field, col_type: &ColumnType) -> DatasetCell {
    use parquet::record::Field;

    match col_type {
        ColumnType::Image => {
            // Image is a Group with a "bytes" sub-field
            if let Field::Group(row) = field {
                for (name, sub_field) in row.get_column_iter() {
                    if name == "bytes" {
                        if let Field::Bytes(bytes) = sub_field {
                            // Resize and encode as base64 thumbnail
                            let raw = bytes.data();
                            if let Ok(img) = image::load_from_memory(raw) {
                                let thumb = img.thumbnail(64, 64);
                                let mut buf = std::io::Cursor::new(Vec::new());
                                if thumb.write_to(&mut buf, image::ImageFormat::Png).is_ok() {
                                    use base64::Engine;
                                    let b64 = base64::engine::general_purpose::STANDARD
                                        .encode(buf.into_inner());
                                    return DatasetCell::Image(format!("data:image/png;base64,{b64}"));
                                }
                            }
                        }
                    }
                }
            }
            DatasetCell::Text("(image)".to_string())
        }
        ColumnType::Number => {
            let val = match field {
                Field::Int(v) => v.to_string(),
                Field::Long(v) => v.to_string(),
                Field::Float(v) => format!("{v:.4}"),
                Field::Double(v) => format!("{v:.4}"),
                Field::UInt(v) => v.to_string(),
                Field::ULong(v) => v.to_string(),
                other => format!("{other}"),
            };
            DatasetCell::Number(val)
        }
        ColumnType::Text => {
            let val = match field {
                Field::Str(s) => s.clone(),
                Field::Bytes(b) => String::from_utf8_lossy(b.data()).to_string(),
                other => format!("{other}"),
            };
            DatasetCell::Text(val)
        }
    }
}
