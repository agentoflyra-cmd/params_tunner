# ROS 2 参数工作台

一个基于 **Tauri + React + Rust** 的 ROS 2 参数可视化编辑器，配合 **Python rclpy sidecar** 与 ROS 2 进行参数通信。

## 架构

```
Tauri Web UI (React + TypeScript)
    │ Tauri IPC (invoke)
    ▼
Rust Backend (参数模型、状态管理、YAML导出)
    │ JSON-RPC over stdin/stdout
    ▼
Python Agent (rclpy: 节点发现 + 参数服务 + /parameter_events)
    │ ROS 2 services
    ▼
ROS 2 Nodes
```

## 功能

- **自动发现** — 扫描 ROS 2 图中的所有节点，筛选具有参数服务的节点
- **参数查看** — 查看参数名称、类型、当前值、描述和约束
- **按类型生成控件** — bool → Switch，integer/double → SpinBox + Slider（有范围时），string → TextInput
- **参数分组** — 按 `.` 前缀自动组织成参数组树（如 `FollowPath.max_vel_x`）
- **修改参数** — 暂存修改后提交，支持批量提交
- **修改追踪** — 标记已修改参数、显示提交成功/失败原因、一键撤销
- **订阅 /parameter_events** — 检测外部参数变更并标记冲突
- **导出 YAML** — 生成标准 ROS 2 参数 YAML，支持：
  - 仅导出修改过的参数
  - 过滤系统内部参数
  - 按 namespace / 按节点分文件
  - 复制到剪贴板或下载文件

## 快速开始

### 前置依赖

- Node.js >= 18
- Rust >= 1.70
- Python >= 3.8 + rclpy（仅使用 MockBackend 时可省略）
- ROS 2 Humble/Foxy（仅使用 MockBackend 时可省略）

### 安装

```bash
cd ros2-param-tuner
npm install
```

### 开发模式（使用 MockBackend — 无需 ROS 2）

```bash
npm run tauri dev
```

### 使用真实 ROS 2

确保 ROS 2 环境已 source，且 Python agent 有执行权限：

```bash
source /opt/ros/humble/setup.bash
npm run tauri dev
```

Tauri 会自动启动 `agent/main.py` 作为 sidecar。

## 项目结构

```
ros2-param-tuner/
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # Tauri 入口
│   │   ├── lib.rs                # 模块导出 + Tauri commands
│   │   ├── parameter_model.rs    # 数据结构 (ParameterEntry, ParameterValue...)
│   │   ├── yaml_exporter.rs      # 标准 ROS 2 YAML 导出
│   │   └── agent_client.rs       # JSON-RPC IPC 客户端 + MockBackend
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                          # Web UI (React + TypeScript)
│   ├── main.tsx                  # 入口
│   ├── App.tsx                   # 主应用组件
│   ├── types.ts                  # TypeScript 类型定义 + 工具函数
│   ├── hooks/
│   │   └── useParameterBackend.ts # Tauri IPC hook
│   ├── components/
│   │   ├── NodeTree.tsx          # 节点树（按 namespace 分组）
│   │   ├── ParameterEditor.tsx   # 参数编辑器（按类型生成控件）
│   │   ├── ParameterGroup.tsx    # 参数分组视图
│   │   ├── ChangeIndicator.tsx   # 修改状态标记
│   │   └── ExportDialog.tsx      # 导出对话框
│   └── styles/
│       └── app.css               # 暗色主题样式
├── agent/                        # ROS 2 Python sidecar
│   ├── __init__.py
│   ├── main.py                   # 入口 + stdin/stdout JSON-RPC IPC
│   ├── node_discovery.py         # 节点发现 + 参数服务检查
│   ├── parameter_client.py       # 参数读写封装
│   └── requirements.txt
├── package.json
├── tsconfig.json
├── vite.config.ts
└── index.html
```

## 导出格式

标准 ROS 2 YAML 格式（与 `ros2 param dump` 兼容）：

```yaml
/navigation/controller_server:
  ros__parameters:
    controller_frequency: 20.0
    FollowPath.max_vel_x: 0.55
    FollowPath.max_vel_theta: 1.0
    debug_trajectory_details: true

/navigation/planner_server:
  ros__parameters:
    expected_planner_frequency: 5.0
    GridBased.tolerance: 0.5
```

## 开发

### 添加新的参数后端

实现 `ParameterBackend` trait 即可替换通信层：

```rust
#[async_trait::async_trait]
impl ParameterBackend for MyCustomBackend {
    async fn discover_targets(&self) -> Result<Vec<ParameterTargetSummary>> { ... }
    async fn inspect_target(&self, node: &str) -> Result<ParameterTarget> { ... }
    // ...
}
```

内置后端：
- `AgentClient` — 通过 JSON-RPC 与 Python agent 通信
- `MockBackend` — 演示/测试用，无需 ROS 2

### Rust 单元测试

```bash
cd src-tauri && cargo test
```

## 许可证

MIT
