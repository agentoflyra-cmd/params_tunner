mod agent_client;
mod parameter_model;
mod yaml_exporter;

use agent_client::AgentClient;
use parameter_model::{
    ExportSelection, ParameterBackend, ParameterTarget, ParameterTargetSummary, ParameterUpdate,
};
use std::sync::Arc;
use tauri::State;

/// 应用状态 — 持有后端实现
pub struct AppState {
    backend: Box<dyn ParameterBackend + Send + Sync>,
}

// region: Tauri Commands

#[tauri::command]
async fn discover_targets(
    state: State<'_, AppState>,
) -> Result<Vec<ParameterTargetSummary>, String> {
    state.backend.discover_targets().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn inspect_target(
    state: State<'_, AppState>,
    full_node_name: String,
) -> Result<ParameterTarget, String> {
    state
        .backend
        .inspect_target(&full_node_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_parameter(
    state: State<'_, AppState>,
    node: String,
    name: String,
    value: parameter_model::ParameterValue,
) -> Result<parameter_model::ApplyResult, String> {
    let update = ParameterUpdate { name, value };
    state
        .backend
        .set_parameter(&node, update)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_parameters(
    state: State<'_, AppState>,
    node: String,
    parameters: Vec<ParameterUpdate>,
    atomic: bool,
) -> Result<Vec<parameter_model::ApplyResult>, String> {
    state
        .backend
        .set_parameters(&node, parameters, atomic)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_yaml(
    state: State<'_, AppState>,
    selection: ExportSelection,
) -> Result<String, String> {
    state
        .backend
        .export_parameters(selection)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_backend_info(state: State<'_, AppState>) -> Result<String, String> {
    // 返回后端类型信息（仅调试用）
    Ok("MockBackend (use --real-agent to connect to ROS 2)".to_string())
}

// endregion

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 尝试启动真实 agent，失败则回退到 MockBackend
            let backend: Box<dyn ParameterBackend + Send + Sync> = {
                let agent_path = app
                    .path_resolver()
                    .resolve_resource("../agent/main.py")
                    .or_else(|| app.path_resolver().resolve_resource("agent/main.py"));

                match agent_path {
                    Some(path) if path.exists() => {
                        match AgentClient::spawn(path.to_str().unwrap_or("")) {
                            Ok(client) => {
                                eprintln!("ROS 2 Agent connected");
                                Box::new(client)
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to start ROS 2 agent ({}), using mock backend",
                                    e
                                );
                                Box::new(agent_client::MockBackend)
                            }
                        }
                    }
                    _ => {
                        eprintln!("ROS 2 agent not found, using mock backend");
                        Box::new(agent_client::MockBackend)
                    }
                }
            };

            app.manage(AppState { backend });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            discover_targets,
            inspect_target,
            set_parameter,
            set_parameters,
            export_yaml,
            get_backend_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
