import React, { useMemo } from "react";
import { ParameterTargetSummary, ParameterTargetState } from "../types";

interface NodeTreeProps {
  targets: ParameterTargetSummary[];
  selectedNode: string | null;
  onSelect: (fullNodeName: string) => void;
}

/** Group targets by namespace for tree display */
interface NamespaceGroup {
  namespace: string;
  nodes: ParameterTargetSummary[];
}

function groupByNamespace(targets: ParameterTargetSummary[]): NamespaceGroup[] {
  const map = new Map<string, ParameterTargetSummary[]>();
  for (const t of targets) {
    const ns = t.namespace || "/";
    if (!map.has(ns)) {
      map.set(ns, []);
    }
    map.get(ns)!.push(t);
  }

  // Sort by namespace
  return Array.from(map.entries())
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([namespace, nodes]) => ({
      namespace,
      nodes: nodes.sort((a, b) => a.node_name.localeCompare(b.node_name)),
    }));
}

function stateIcon(state: ParameterTargetState): string {
  switch (state) {
    case "Available":
      return "🟢";
    case "NoParameterService":
      return "🟡";
    case "Timeout":
      return "🔴";
    case "LifecycleInactive":
      return "🟠";
    case "Disappeared":
      return "⚫";
    default:
      return "⚪";
  }
}

function stateLabel(state: ParameterTargetState): string {
  switch (state) {
    case "Available":
      return "可用";
    case "NoParameterService":
      return "无参数服务";
    case "Timeout":
      return "超时";
    case "LifecycleInactive":
      return "未激活";
    case "Disappeared":
      return "已消失";
    default:
      return "未知";
  }
}

export function NodeTree({ targets, selectedNode, onSelect }: NodeTreeProps) {
  const groups = useMemo(() => groupByNamespace(targets), [targets]);

  if (targets.length === 0) {
    return (
      <div className="node-tree-empty">
        <p>未发现 ROS 2 节点</p>
        <p className="hint">点击"刷新"扫描 ROS 2 图</p>
      </div>
    );
  }

  return (
    <div className="node-tree">
      {groups.map((group) => (
        <div key={group.namespace} className="namespace-group">
          <div className="namespace-header">
            <span className="namespace-icon">📁</span>
            <span className="namespace-name">{group.namespace}</span>
            <span className="namespace-count">{group.nodes.length}</span>
          </div>
          <div className="namespace-nodes">
            {group.nodes.map((node) => (
              <div
                key={node.full_node_name}
                className={`node-item ${
                  selectedNode === node.full_node_name ? "selected" : ""
                } ${node.state !== "Available" ? "unavailable" : ""}`}
                onClick={() => onSelect(node.full_node_name)}
                title={`${node.full_node_name} (${stateLabel(node.state)})`}
              >
                <span className="node-state-icon">{stateIcon(node.state)}</span>
                <span className="node-name">{node.node_name}</span>
                <span className="node-param-count">{node.parameter_count}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
