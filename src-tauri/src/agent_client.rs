use crate::parameter_model::{
    ApplyResult, ParameterBackend, ParameterTarget, ParameterTargetState,
    ParameterTargetSummary, ParameterUpdate,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

/// JSON-RPC 请求
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// JSON-RPC 响应
#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// Agent 客户端 — 通过 stdin/stdout 与 Python agent 通信
pub struct AgentClient {
    child: Mutex<Child>,
    writer: Mutex<Box<dyn Write + Send>>,
    reader: Mutex<BufReader<Box<dyn std::io::Read + Send>>>,
    next_id: Mutex<u64>,
    agent_path: String,
}

impl AgentClient {
    /// 启动 agent 进程
    pub fn spawn(agent_path: &str) -> Result<Self> {
        let mut child = Command::new("python3")
            .arg(agent_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn ROS 2 parameter agent")?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to capture agent stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("Failed to capture agent stdout")?;

        Ok(Self {
            child: Mutex::new(child),
            writer: Mutex::new(Box::new(stdin)),
            reader: Mutex::new(BufReader::new(Box::new(stdout))),
            next_id: Mutex::new(0),
            agent_path: agent_path.to_string(),
        })
    }

    /// 发送 JSON-RPC 请求并等待响应
    fn call(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let id = *next_id;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize request")?;

        // 写入请求（每行一个 JSON）
        {
            let mut writer = self.writer.lock().unwrap();
            writeln!(writer, "{}", request_json).context("Failed to write to agent stdin")?;
            writer.flush().context("Failed to flush agent stdin")?;
        }

        // 读取响应（每行一个 JSON）
        let mut line = String::new();
        {
            let mut reader = self.reader.lock().unwrap();
            reader
                .read_line(&mut line)
                .context("Failed to read agent response")?;
        }

        let response: JsonRpcResponse =
            serde_json::from_str(&line).context("Failed to parse agent response")?;

        if let Some(error) = response.error {
            anyhow::bail!("Agent error: {} (code {})", error.message, error.code);
        }

        response
            .result
            .context("Agent response missing result field")
    }

    /// 检查 agent 是否仍在运行
    pub fn is_alive(&self) -> bool {
        let mut child = self.child.lock().unwrap();
        match child.try_wait() {
            Ok(Some(_)) => false, // 已退出
            Ok(None) => true,     // 仍在运行
            Err(_) => false,
        }
    }

    /// 重启 agent
    pub fn restart(&self) -> Result<()> {
        // 先尝试优雅终止
        let _ = self.call("shutdown", None);
        std::thread::sleep(Duration::from_millis(200));

        // 如果还活着就杀掉
        let mut child = self.child.lock().unwrap();
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
        drop(child);

        // 重新 spawn
        let new = Self::spawn(&self.agent_path)?;
        *self.child.lock().unwrap() = new.child.into_inner().unwrap();
        *self.writer.lock().unwrap() = new.writer.into_inner().unwrap();
        *self.reader.lock().unwrap() = new.reader.into_inner().unwrap();
        Ok(())
    }
}

impl Drop for AgentClient {
    fn drop(&mut self) {
        let _ = self.call("shutdown", None);
        let mut child = self.child.lock().unwrap();
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[async_trait::async_trait]
impl ParameterBackend for AgentClient {
    async fn discover_targets(&self) -> Result<Vec<ParameterTargetSummary>> {
        let value = self.call("discover_targets", None)?;
        serde_json::from_value(value).context("Failed to parse discover_targets response")
    }

    async fn inspect_target(&self, full_node_name: &str) -> Result<ParameterTarget> {
        let params = serde_json::json!({ "node": full_node_name });
        let value = self.call("inspect_target", Some(params))?;
        serde_json::from_value(value).context("Failed to parse inspect_target response")
    }

    async fn set_parameter(&self, node: &str, update: ParameterUpdate) -> Result<ApplyResult> {
        let params = serde_json::json!({
            "node": node,
            "name": update.name,
            "value": update.value,
        });
        let value = self.call("set_parameter", Some(params))?;
        serde_json::from_value(value).context("Failed to parse set_parameter response")
    }

    async fn set_parameters(
        &self,
        node: &str,
        updates: Vec<ParameterUpdate>,
        atomic: bool,
    ) -> Result<Vec<ApplyResult>> {
        let params = serde_json::json!({
            "node": node,
            "parameters": updates,
            "atomic": atomic,
        });
        let value = self.call("set_parameters", Some(params))?;
        serde_json::from_value(value).context("Failed to parse set_parameters response")
    }

    async fn export_parameters(&self, selection: crate::parameter_model::ExportSelection) -> Result<String> {
        // 对于 agent 导出的情况，先获取所有目标，再在 Rust 侧生成 YAML
        let targets = self.discover_targets().await?;
        let mut full_targets = Vec::new();
        for summary in &targets {
            if summary.state == ParameterTargetState::Available {
                match self.inspect_target(&summary.full_node_name).await {
                    Ok(t) => full_targets.push(t),
                    Err(e) => {
                        eprintln!("Warning: failed to inspect {}: {}", summary.full_node_name, e);
                    }
                }
            }
        }
        let yaml = crate::yaml_exporter::export_all_to_yaml(&full_targets, &selection);
        Ok(yaml)
    }
}

/// MockBackend — 无 ROS 环境时的测试/演示后端
pub struct MockBackend;

#[async_trait::async_trait]
impl ParameterBackend for MockBackend {
    async fn discover_targets(&self) -> Result<Vec<ParameterTargetSummary>> {
        Ok(vec![
            ParameterTargetSummary {
                full_node_name: "/navigation/controller_server".to_string(),
                namespace: "/navigation".to_string(),
                node_name: "controller_server".to_string(),
                state: ParameterTargetState::Available,
                parameter_count: 12,
            },
            ParameterTargetSummary {
                full_node_name: "/navigation/planner_server".to_string(),
                namespace: "/navigation".to_string(),
                node_name: "planner_server".to_string(),
                state: ParameterTargetState::Available,
                parameter_count: 8,
            },
            ParameterTargetSummary {
                full_node_name: "/localization/amcl".to_string(),
                namespace: "/localization".to_string(),
                node_name: "amcl".to_string(),
                state: ParameterTargetState::Available,
                parameter_count: 15,
            },
        ])
    }

    async fn inspect_target(&self, full_node_name: &str) -> Result<ParameterTarget> {
        use crate::parameter_model::{ApplyState, ParameterEntry, ParameterValue};

        let (namespace, node_name) = match full_node_name.rsplit_once('/') {
            Some((ns, name)) => {
                if ns.is_empty() {
                    ("/".to_string(), name.to_string())
                } else {
                    (ns.to_string(), name.to_string())
                }
            }
            None => ("/".to_string(), full_node_name.to_string()),
        };

        let mock_params = match node_name.as_str() {
            "controller_server" => vec![
                ParameterEntry {
                    name: "controller_frequency".to_string(),
                    current_value: ParameterValue::Double(20.0),
                    original_value: ParameterValue::Double(20.0),
                    description: "Controller update frequency".to_string(),
                    additional_constraints: "Range: [0.0, 100.0]".to_string(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "FollowPath.max_vel_x".to_string(),
                    current_value: ParameterValue::Double(0.55),
                    original_value: ParameterValue::Double(0.55),
                    description: "Maximum velocity in X direction".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "FollowPath.max_vel_theta".to_string(),
                    current_value: ParameterValue::Double(1.0),
                    original_value: ParameterValue::Double(1.0),
                    description: "Maximum angular velocity".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "debug_trajectory_details".to_string(),
                    current_value: ParameterValue::Bool(true),
                    original_value: ParameterValue::Bool(true),
                    description: "Enable debug trajectory details".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "use_sim_time".to_string(),
                    current_value: ParameterValue::Bool(false),
                    original_value: ParameterValue::Bool(false),
                    description: "Use simulation time".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
            ],
            "planner_server" => vec![
                ParameterEntry {
                    name: "expected_planner_frequency".to_string(),
                    current_value: ParameterValue::Double(5.0),
                    original_value: ParameterValue::Double(5.0),
                    description: "Expected planner update frequency".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "GridBased.tolerance".to_string(),
                    current_value: ParameterValue::Double(0.5),
                    original_value: ParameterValue::Double(0.5),
                    description: "Planning tolerance".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
            ],
            "amcl" => vec![
                ParameterEntry {
                    name: "min_particles".to_string(),
                    current_value: ParameterValue::Integer(500),
                    original_value: ParameterValue::Integer(500),
                    description: "Minimum number of particles".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "max_particles".to_string(),
                    current_value: ParameterValue::Integer(2000),
                    original_value: ParameterValue::Integer(2000),
                    description: "Maximum number of particles".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "update_min_a".to_string(),
                    current_value: ParameterValue::Double(0.2),
                    original_value: ParameterValue::Double(0.2),
                    description: "Minimum angular movement to trigger update".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
            ],
            _ => vec![],
        };

        Ok(ParameterTarget {
            full_node_name: full_node_name.to_string(),
            namespace,
            node_name,
            state: ParameterTargetState::Available,
            parameters: mock_params,
        })
    }

    async fn set_parameter(&self, node: &str, update: ParameterUpdate) -> Result<ApplyResult> {
        // Mock: 总是成功
        Ok(ApplyResult {
            name: update.name,
            successful: true,
            reason: None,
        })
    }

    async fn set_parameters(
        &self,
        _node: &str,
        updates: Vec<ParameterUpdate>,
        _atomic: bool,
    ) -> Result<Vec<ApplyResult>> {
        Ok(updates
            .into_iter()
            .map(|u| ApplyResult {
                name: u.name,
                successful: true,
                reason: None,
            })
            .collect())
    }

    async fn export_parameters(
        &self,
        selection: crate::parameter_model::ExportSelection,
    ) -> Result<String> {
        let summaries = self.discover_targets().await?;
        let mut targets = Vec::new();
        for s in summaries {
            if let Ok(t) = self.inspect_target(&s.full_node_name).await {
                targets.push(t);
            }
        }
        Ok(crate::yaml_exporter::export_all_to_yaml(&targets, &selection))
    }
}
