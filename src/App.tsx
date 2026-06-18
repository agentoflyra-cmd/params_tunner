import React, { useEffect, useState, useCallback } from "react";
import { useParameterBackend } from "./hooks/useParameterBackend";
import { NodeTree } from "./components/NodeTree";
import { ParameterGroupView } from "./components/ParameterGroup";
import { ExportDialog } from "./components/ExportDialog";

function App() {
  const {
    loading,
    error,
    targets,
    currentTarget,
    discover,
    inspect,
    stageParameter,
    applyParameter,
    applyAllPending,
    resetParameter,
    exportYaml,
    getModifiedCount,
    setError,
  } = useParameterBackend();

  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [showExport, setShowExport] = useState(false);
  const [lastAction, setLastAction] = useState<string | null>(null);

  // Auto-discover on mount
  useEffect(() => {
    discover();
  }, [discover]);

  const handleSelectNode = useCallback(
    async (fullNodeName: string) => {
      setSelectedNode(fullNodeName);
      await inspect(fullNodeName);
    },
    [inspect]
  );

  const handleStage = useCallback(
    (name: string, value: any) => {
      stageParameter(name, value);
      setLastAction(`暂存修改: ${name}`);
    },
    [stageParameter]
  );

  const handleApply = useCallback(
    async (name: string) => {
      if (!currentTarget) return;
      const result = await applyParameter(currentTarget.full_node_name, name);
      if (result?.successful) {
        setLastAction(`已应用: ${name}`);
      } else {
        setLastAction(`应用失败: ${name} — ${result?.reason || "未知错误"}`);
      }
    },
    [currentTarget, applyParameter]
  );

  const handleApplyAll = useCallback(async () => {
    if (!currentTarget) return;
    const results = await applyAllPending();
    const successCount = results.filter((r) => r.successful).length;
    const failCount = results.filter((r) => !r.successful).length;
    setLastAction(`批量应用: ${successCount} 成功, ${failCount} 失败`);
  }, [currentTarget, applyAllPending]);

  const handleReset = useCallback(
    (name: string) => {
      resetParameter(name);
      setLastAction(`撤销: ${name}`);
    },
    [resetParameter]
  );

  const modifiedCount = getModifiedCount();
  const pendingCount = currentTarget?.parameters.filter(
    (p) => p.apply_state === "Pending"
  ).length ?? 0;

  return (
    <div className="app">
      {/* Top bar */}
      <header className="topbar">
        <div className="topbar-left">
          <h1 className="app-title">ROS 2 参数工作台</h1>
        </div>
        <div className="topbar-center">
          {lastAction && (
            <span className="last-action">{lastAction}</span>
          )}
          {error && (
            <span className="error-toast" onClick={() => setError(null)}>
              ⚠ {error}
            </span>
          )}
        </div>
        <div className="topbar-right">
          {modifiedCount > 0 && (
            <span className="modified-badge">{modifiedCount} 项修改</span>
          )}
          <button
            className="btn btn-primary"
            onClick={() => discover()}
            disabled={loading}
          >
            {loading ? "扫描中..." : "🔄 刷新"}
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => setShowExport(true)}
            disabled={targets.length === 0}
          >
            📥 导出 YAML
          </button>
        </div>
      </header>

      {/* Main layout */}
      <div className="main-layout">
        {/* Left sidebar — node tree */}
        <aside className="sidebar">
          <div className="sidebar-header">
            <h2>节点</h2>
            <span className="count-badge">{targets.length}</span>
          </div>
          <NodeTree
            targets={targets}
            selectedNode={selectedNode}
            onSelect={handleSelectNode}
          />
        </aside>

        {/* Main content — parameter editor */}
        <main className="content">
          {loading && !currentTarget ? (
            <div className="loading-state">
              <div className="spinner" />
              <p>扫描 ROS 2 图...</p>
            </div>
          ) : currentTarget ? (
            <div className="param-panel">
              <div className="panel-header">
                <div className="panel-title">
                  <h2>{currentTarget.full_node_name}</h2>
                  <span className="param-count">
                    {currentTarget.parameters.length} 个参数
                  </span>
                </div>
                <div className="panel-actions">
                  {pendingCount > 0 && (
                    <button className="btn btn-primary" onClick={handleApplyAll}>
                      批量应用 ({pendingCount})
                    </button>
                  )}
                  <button
                    className="btn btn-ghost"
                    onClick={() => handleSelectNode(currentTarget.full_node_name)}
                  >
                    🔄 刷新
                  </button>
                </div>
              </div>

              {currentTarget.parameters.length === 0 ? (
                <div className="empty-state">
                  <p>该节点没有可用参数</p>
                </div>
              ) : (
                <ParameterGroupView
                  parameters={currentTarget.parameters}
                  onStage={handleStage}
                  onApply={handleApply}
                  onReset={handleReset}
                />
              )}
            </div>
          ) : (
            <div className="empty-state">
              <div className="empty-icon">🔧</div>
              <h2>选择一个节点</h2>
              <p>从左侧节点树选择一个 ROS 2 节点以查看和编辑参数</p>
              <button className="btn btn-primary" onClick={() => discover()}>
                🔄 扫描 ROS 2 节点
              </button>
            </div>
          )}
        </main>
      </div>

      {/* Export dialog */}
      {showExport && (
        <ExportDialog
          targets={targets}
          onExport={exportYaml}
          onClose={() => setShowExport(false)}
        />
      )}
    </div>
  );
}

export default App;
