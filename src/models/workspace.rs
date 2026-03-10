use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub dataset_id: String,
    pub dataset_name: String,
    pub code: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub plots: Vec<String>,
    pub table_html: Option<String>,
    #[serde(default)]
    pub xai_plots: Vec<String>,
    #[serde(default)]
    pub xai_html: Option<String>,
}
