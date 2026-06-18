use serde::{Deserialize, Serialize};

/// 参数目标（节点）的状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterTargetState {
    /// 正常可用
    Available,
    /// 节点存在但不提供参数服务
    NoParameterService,
    /// 请求超时
    Timeout,
    /// Lifecycle 节点处于非活跃状态
    LifecycleInactive,
    /// 节点已消失
    Disappeared,
}

/// 参数应用状态（本地追踪）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApplyState {
    /// 未修改
    Unchanged,
    /// 已修改但未提交
    Pending,
    /// 提交成功
    Applied,
    /// 提交失败（携带原因）
    Failed(String),
    /// 与外部变更冲突
    Conflict,
}

/// ROS 2 参数值
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParameterValue {
    NotSet,
    Bool(bool),
    Integer(i64),
    Double(f64),
    String(String),
    ByteArray(Vec<u8>),
    BoolArray(Vec<bool>),
    IntegerArray(Vec<i64>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
}

impl ParameterValue {
    /// 返回值的类型名称（用于 UI 展示和 YAML 序列化）
    pub fn type_name(&self) -> &'static str {
        match self {
            ParameterValue::NotSet => "not_set",
            ParameterValue::Bool(_) => "bool",
            ParameterValue::Integer(_) => "integer",
            ParameterValue::Double(_) => "double",
            ParameterValue::String(_) => "string",
            ParameterValue::ByteArray(_) => "byte_array",
            ParameterValue::BoolArray(_) => "bool_array",
            ParameterValue::IntegerArray(_) => "integer_array",
            ParameterValue::DoubleArray(_) => "double_array",
            ParameterValue::StringArray(_) => "string_array",
        }
    }

    /// 展示值（用于 UI 文本显示）
    pub fn display_value(&self) -> String {
        match self {
            ParameterValue::NotSet => "<not set>".to_string(),
            ParameterValue::Bool(v) => v.to_string(),
            ParameterValue::Integer(v) => v.to_string(),
            ParameterValue::Double(v) => format_gently(v),
            ParameterValue::String(v) => v.clone(),
            ParameterValue::ByteArray(v) => format!("[{} bytes]", v.len()),
            ParameterValue::BoolArray(v) => format!("[{:?}]", v),
            ParameterValue::IntegerArray(v) => format!("{:?}", v),
            ParameterValue::DoubleArray(v) => format!("{:?}", v),
            ParameterValue::StringArray(v) => format!("{:?}", v),
        }
    }

    /// 是否为数组类型
    pub fn is_array(&self) -> bool {
        matches!(
            self,
            ParameterValue::ByteArray(_)
                | ParameterValue::BoolArray(_)
                | ParameterValue::IntegerArray(_)
                | ParameterValue::DoubleArray(_)
                | ParameterValue::StringArray(_)
        )
    }

    /// 转换为 YAML 标量值表示
    pub fn to_yaml_value(&self) -> serde_json::Value {
        match self {
            ParameterValue::NotSet => serde_json::Value::Null,
            ParameterValue::Bool(v) => serde_json::Value::Bool(*v),
            ParameterValue::Integer(v) => serde_json::json!(*v),
            ParameterValue::Double(v) => serde_json::json!(*v),
            ParameterValue::String(v) => serde_json::Value::String(v.clone()),
            ParameterValue::ByteArray(v) => serde_json::json!(v),
            ParameterValue::BoolArray(v) => serde_json::json!(v),
            ParameterValue::IntegerArray(v) => serde_json::json!(v),
            ParameterValue::DoubleArray(v) => serde_json::json!(v),
            ParameterValue::StringArray(v) => serde_json::json!(v),
        }
    }
}

fn format_gently(v: &f64) -> String {
    if v.fract() == 0.0 && v.is_finite() {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

/// 整数范围约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerRange {
    pub from: i64,
    pub to: i64,
    pub step: u64,
}

/// 浮点范围约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingPointRange {
    pub from: f64,
    pub to: f64,
    pub step: f64,
}

/// 单个参数条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterEntry {
    /// 参数全名（如 "FollowPath.max_vel_x"）
    pub name: String,
    /// 当前值
    pub current_value: ParameterValue,
    /// 原始值（用于比较和撤销）
    pub original_value: ParameterValue,

    /// 描述文本
    pub description: String,
    /// 附加约束描述
    pub additional_constraints: String,

    /// 是否只读
    pub read_only: bool,
    /// 是否允许动态改变类型
    pub dynamic_typing: bool,

    /// 整数范围约束
    pub integer_ranges: Vec<IntegerRange>,
    /// 浮点范围约束
    pub floating_ranges: Vec<FloatingPointRange>,

    /// 是否已被修改（本地追踪）
    pub changed: bool,
    /// 应用状态
    pub apply_state: ApplyState,
}

impl ParameterEntry {
    /// 重置为原始值
    pub fn reset(&mut self) {
        self.current_value = self.original_value.clone();
        self.changed = false;
        self.apply_state = ApplyState::Unchanged;
    }

    /// 应用新值（本地暂存，不发送到 ROS）
    pub fn stage_value(&mut self, value: ParameterValue) {
        self.current_value = value;
        self.changed = self.current_value != self.original_value;
        if self.changed {
            self.apply_state = ApplyState::Pending;
        } else {
            self.apply_state = ApplyState::Unchanged;
        }
    }
}

/// 参数目标（一个 ROS 节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterTarget {
    /// 完整节点名（如 "/navigation/controller_server"）
    pub full_node_name: String,
    /// 命名空间（如 "/navigation"）
    pub namespace: String,
    /// 节点名（如 "controller_server"）
    pub node_name: String,
    /// 状态
    pub state: ParameterTargetState,
    /// 参数列表
    pub parameters: Vec<ParameterEntry>,
}

impl ParameterTarget {
    /// 获取所有已修改的参数
    pub fn changed_parameters(&self) -> Vec<&ParameterEntry> {
        self.parameters.iter().filter(|p| p.changed).collect()
    }

    /// 判断是否有未提交的修改
    pub fn has_pending_changes(&self) -> bool {
        self.parameters
            .iter()
            .any(|p| matches!(p.apply_state, ApplyState::Pending))
    }
}

/// 参数目标摘要（用于列表展示，不包含参数详情）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterTargetSummary {
    pub full_node_name: String,
    pub namespace: String,
    pub node_name: String,
    pub state: ParameterTargetState,
    pub parameter_count: usize,
}

/// 参数更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterUpdate {
    pub name: String,
    pub value: ParameterValue,
}

/// 单参数应用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub name: String,
    pub successful: bool,
    pub reason: Option<String>,
}

/// 导出选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSelection {
    /// 要导出的节点全名列表（空 = 全部）
    pub nodes: Option<Vec<String>>,
    /// 仅导出已修改的参数
    pub only_changed: bool,
    /// 包含系统内部参数（use_sim_time, qos_overrides.* 等）
    pub include_internal: bool,
    /// 包含只读参数
    pub include_readonly: bool,
    /// 按 namespace 分文件
    pub split_by_namespace: bool,
    /// 按节点分文件
    pub split_by_node: bool,
    /// 每文件最大节点数（split_by_namespace 时未指定 namespace 的节点放到 root.yaml）
    pub max_nodes_per_file: Option<usize>,
}

impl Default for ExportSelection {
    fn default() -> Self {
        Self {
            nodes: None,
            only_changed: true,
            include_internal: false,
            include_readonly: false,
            split_by_namespace: false,
            split_by_node: false,
            max_nodes_per_file: None,
        }
    }
}

/// 判断是否为 ROS 2 内部参数
pub fn is_internal_parameter(name: &str) -> bool {
    name == "use_sim_time"
        || name == "start_type_description_service"
        || name.starts_with("qos_overrides.")
}

/// ParameterBackend trait — 后端抽象
#[async_trait::async_trait]
pub trait ParameterBackend: Send + Sync {
    /// 发现所有可调参的目标
    async fn discover_targets(&self) -> anyhow::Result<Vec<ParameterTargetSummary>>;

    /// 检查单个节点的详细参数
    async fn inspect_target(
        &self,
        full_node_name: &str,
    ) -> anyhow::Result<ParameterTarget>;

    /// 设置单个参数
    async fn set_parameter(
        &self,
        node: &str,
        update: ParameterUpdate,
    ) -> anyhow::Result<ApplyResult>;

    /// 批量设置参数
    async fn set_parameters(
        &self,
        node: &str,
        updates: Vec<ParameterUpdate>,
        atomic: bool,
    ) -> anyhow::Result<Vec<ApplyResult>>;

    /// 导出参数为 YAML 字符串
    async fn export_parameters(
        &self,
        selection: ExportSelection,
    ) -> anyhow::Result<String>;
}
