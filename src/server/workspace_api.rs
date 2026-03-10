use dioxus::prelude::*;
use crate::models::{Workspace, ExecutionResult};

#[post("/api/workspaces/list")]
pub async fn list_workspaces_server() -> Result<Vec<Workspace>, ServerFnError> {
    crate::db::list_workspaces().map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[post("/api/workspaces/get")]
pub async fn get_workspace_server(id: String) -> Result<Option<Workspace>, ServerFnError> {
    crate::db::get_workspace(&id).map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[post("/api/workspaces/create")]
pub async fn create_workspace_server(
    name: String,
    dataset_id: String,
) -> Result<Workspace, ServerFnError> {
    let dataset = crate::db::get_dataset(&dataset_id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Dataset not found".to_string()))?;

    let now = chrono::Utc::now().to_rfc3339();
    let ws = Workspace {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        dataset_id,
        dataset_name: dataset.name,
        code: String::new(),
        created_at: now.clone(),
        updated_at: now,
    };

    crate::db::insert_workspace(&ws)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;

    Ok(ws)
}

#[post("/api/workspaces/save")]
pub async fn save_workspace_code_server(id: String, code: String) -> Result<(), ServerFnError> {
    crate::db::update_workspace_code(&id, &code)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[post("/api/workspaces/delete")]
pub async fn delete_workspace_server(id: String) -> Result<(), ServerFnError> {
    crate::db::delete_workspace_db(&id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))
}

#[post("/api/workspaces/run")]
pub async fn run_python_server(workspace_id: String, code: String) -> Result<ExecutionResult, ServerFnError> {
    let result = run_python_impl(workspace_id.clone(), code).await?;
    // Persist the result so it can be restored later
    #[cfg(feature = "server")]
    {
        if let Ok(json) = serde_json::to_string(&result) {
            let _ = crate::db::save_workspace_run_result(&workspace_id, &json);
        }
    }
    Ok(result)
}

#[post("/api/workspaces/last-run-result")]
pub async fn get_last_run_result_server(workspace_id: String) -> Result<Option<ExecutionResult>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let json_opt = crate::db::get_workspace_run_result(&workspace_id)
            .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?;
        match json_opt {
            Some(json) => {
                let result: ExecutionResult = serde_json::from_str(&json)
                    .map_err(|e| ServerFnError::new(format!("Parse error: {e}")))?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
    #[cfg(not(feature = "server"))]
    {
        Ok(None)
    }
}

#[cfg(feature = "server")]
async fn run_python_impl(workspace_id: String, code: String) -> Result<ExecutionResult, ServerFnError> {
    use std::io::Write;

    // Look up workspace to get dataset_id
    let ws = crate::db::get_workspace(&workspace_id)
        .map_err(|e| ServerFnError::new(format!("DB error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Workspace not found".to_string()))?;

    // Find parquet files for the dataset
    let base_dir = format!("data/datasets/{}", ws.dataset_id);
    let base_path = std::path::Path::new(&base_dir);

    let mut parquet_files: Vec<String> = Vec::new();
    if base_path.exists() {
        collect_parquet_files_recursive(base_path, &mut parquet_files);
    }

    // Convert to absolute paths so Python can find them regardless of cwd
    let parquet_files: Vec<String> = parquet_files
        .iter()
        .filter_map(|p| {
            std::path::Path::new(p)
                .canonicalize()
                .ok()
                .map(|abs| {
                    let s = abs.to_string_lossy().replace('\\', "/");
                    // Strip Windows \\?\ prefix
                    s.strip_prefix("//?/").unwrap_or(&s).to_string()
                })
        })
        .collect();

    // Build the Python runner script
    let parquet_paths_json = serde_json::to_string(&parquet_files).unwrap_or_else(|_| "[]".to_string());

    // Compute absolute path for the dataset directory
    let abs_base = std::path::Path::new(&base_dir)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&base_dir));
    let abs_base_str = abs_base.to_string_lossy().replace('\\', "/");
    // Strip Windows \\?\ prefix
    let abs_base_str = abs_base_str.strip_prefix("//?/").unwrap_or(&abs_base_str).to_string();

    let user_code_escaped = code.replace("'''", r"\'\'\'");

    let runner_script = include_str!("python_runner_template.py")
        .replace("__ABS_BASE_STR__", &abs_base_str)
        .replace("__PARQUET_PATHS_JSON__", &parquet_paths_json)
        .replace("__USER_CODE__", &user_code_escaped);

    // Write temp script file
    let tmp_dir = std::env::temp_dir();
    let script_name = format!("open_fair_flow_run_{}.py", ws.id.replace('-', ""));
    let script_path = tmp_dir.join(&script_name);

    {
        let mut f = std::fs::File::create(&script_path)
            .map_err(|e| ServerFnError::new(format!("Failed to create temp script: {e}")))?;
        f.write_all(runner_script.as_bytes())
            .map_err(|e| ServerFnError::new(format!("Failed to write temp script: {e}")))?;
    }

    // Execute with timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1800),
        tokio::process::Command::new("python")
            .arg(&script_path)
            .output(),
    )
    .await;

    // Cleanup
    let _ = std::fs::remove_file(&script_path);

    match result {
        Ok(Ok(output)) => {
            let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Try to parse the JSON output (delimited by __FEDLAB_RESULT__ marker)
            let marker = "__FEDLAB_RESULT__";
            if let Some(marker_pos) = raw_stdout.rfind(marker) {
                let json_str = &raw_stdout[marker_pos + marker.len()..].trim();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return Ok(ExecutionResult {
                        stdout: parsed["stdout"].as_str().unwrap_or("").to_string(),
                        stderr: if raw_stderr.is_empty() {
                            parsed["stderr"].as_str().unwrap_or("").to_string()
                        } else {
                            format!("{}{}", parsed["stderr"].as_str().unwrap_or(""), raw_stderr)
                        },
                        plots: parsed["plots"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        table_html: parsed["table_html"]
                            .as_str()
                            .map(|s| s.to_string()),
                        xai_plots: parsed["xai_plots"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        xai_html: parsed["xai_html"]
                            .as_str()
                            .map(|s| s.to_string()),
                    });
                }
            }

            // Fallback: return raw output
            Ok(ExecutionResult {
                stdout: raw_stdout,
                stderr: raw_stderr,
                plots: Vec::new(),
                table_html: None,
                xai_plots: Vec::new(),
                xai_html: None,
            })
        }
        Ok(Err(e)) => {
            Ok(ExecutionResult {
                stdout: String::new(),
                stderr: format!("Failed to execute Python: {e}\nMake sure Python is installed and accessible in PATH."),
                plots: Vec::new(),
                table_html: None,
                xai_plots: Vec::new(),
                xai_html: None,
            })
        }
        Err(_) => {
            Ok(ExecutionResult {
                stdout: String::new(),
                stderr: "Execution timed out after 30 minutes.".to_string(),
                plots: Vec::new(),
                table_html: None,
                xai_plots: Vec::new(),
                xai_html: None,
            })
        }
    }
}

#[cfg(feature = "server")]
fn collect_parquet_files_recursive(dir: &std::path::Path, out: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_parquet_files_recursive(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "parquet") {
                if let Some(s) = path.to_str() {
                    out.push(s.replace('\\', "/"));
                }
            }
        }
    }
}
