use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub dataset_type: String,
    pub description: String,
    pub tags: Vec<String>,
    pub format: String,
    pub num_samples: Option<u64>,
    pub num_classes: Option<u32>,
    pub file_size: u64,
    pub source: String,
    pub file_path: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetPreview {
    pub sample_images: Vec<String>,
    pub class_distribution: Vec<(String, u32)>,
    pub summary: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetViewerPage {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<DatasetViewerRow>,
    pub total_rows: u64,
    pub offset: u64,
    pub limit: u64,
    pub split: String,
    pub available_splits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub col_type: ColumnType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnType {
    Image,
    Text,
    Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetViewerRow {
    pub index: u64,
    pub cells: Vec<DatasetCell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatasetCell {
    Image(String),
    Text(String),
    Number(String),
}

impl Dataset {
    pub fn human_readable_size(&self) -> String {
        let size = self.file_size as f64;
        if size < 1024.0 {
            format!("{} B", self.file_size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }

    pub fn status_color(&self) -> &str {
        match self.status.as_str() {
            "ready" => "border border-white text-white",
            "uploading" | "downloading" => "border border-[#888] text-[#888] animate-pulse",
            "error" => "border border-dashed border-[#ff3333] text-[#ff3333]",
            _ => "border border-[#444] text-[#444]",
        }
    }
}
