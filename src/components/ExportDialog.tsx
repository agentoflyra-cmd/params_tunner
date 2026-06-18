import React, { useState } from "react";
import { ExportSelection, ParameterTargetSummary } from "../types";

interface ExportDialogProps {
  targets: ParameterTargetSummary[];
  onExport: (selection: ExportSelection) => Promise<string>;
  onClose: () => void;
}

export function ExportDialog({ targets, onExport, onClose }: ExportDialogProps) {
  const [onlyChanged, setOnlyChanged] = useState(true);
  const [includeInternal, setIncludeInternal] = useState(false);
  const [includeReadonly, setIncludeReadonly] = useState(false);
  const [splitByNamespace, setSplitByNamespace] = useState(false);
  const [splitByNode, setSplitByNode] = useState(false);
  const [yamlOutput, setYamlOutput] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const availableTargets = targets.filter((t) => t.state === "Available");

  const handleExport = async () => {
    setLoading(true);
    setError(null);
    try {
      const selection: ExportSelection = {
        only_changed: onlyChanged,
        include_internal: includeInternal,
        include_readonly: includeReadonly,
        split_by_namespace: splitByNamespace,
        split_by_node: splitByNode,
      };
      const yaml = await onExport(selection);
      setYamlOutput(yaml);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCopy = async () => {
    if (yamlOutput) {
      try {
        await navigator.clipboard.writeText(yamlOutput);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch {
        // Fallback
        const textarea = document.createElement("textarea");
        textarea.value = yamlOutput;
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    }
  };

  const handleDownload = () => {
    if (!yamlOutput) return;
    const blob = new Blob([yamlOutput], { type: "text/yaml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "ros2_params.yaml";
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <h2>导出参数 YAML</h2>
          <button className="btn btn-ghost" onClick={onClose}>
            ✕
          </button>
        </div>

        {!yamlOutput ? (
          <>
            <div className="dialog-body">
              <p className="dialog-info">
                将导出 {availableTargets.length} 个节点的参数
              </p>

              <div className="export-options">
                <label className="export-option">
                  <input
                    type="checkbox"
                    checked={onlyChanged}
                    onChange={(e) => setOnlyChanged(e.target.checked)}
                  />
                  <span>仅导出修改过的参数</span>
                </label>

                <label className="export-option">
                  <input
                    type="checkbox"
                    checked={includeInternal}
                    onChange={(e) => setIncludeInternal(e.target.checked)}
                  />
                  <span>包含系统参数 (use_sim_time, qos_overrides.*)</span>
                </label>

                <label className="export-option">
                  <input
                    type="checkbox"
                    checked={includeReadonly}
                    onChange={(e) => setIncludeReadonly(e.target.checked)}
                  />
                  <span>包含只读参数</span>
                </label>

                <hr />

                <label className="export-option">
                  <input
                    type="checkbox"
                    checked={splitByNamespace}
                    onChange={(e) => setSplitByNamespace(e.target.checked)}
                  />
                  <span>按 namespace 分文件</span>
                </label>

                <label className="export-option">
                  <input
                    type="checkbox"
                    checked={splitByNode}
                    onChange={(e) => setSplitByNode(e.target.checked)}
                  />
                  <span>按节点分文件</span>
                </label>
              </div>

              {error && <div className="export-error">{error}</div>}
            </div>

            <div className="dialog-footer">
              <button className="btn btn-ghost" onClick={onClose}>
                取消
              </button>
              <button
                className="btn btn-primary"
                onClick={handleExport}
                disabled={loading}
              >
                {loading ? "导出中..." : "导出"}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="dialog-body">
              <div className="yaml-preview">
                <pre>{yamlOutput}</pre>
              </div>
            </div>

            <div className="dialog-footer">
              <button className="btn btn-ghost" onClick={onClose}>
                关闭
              </button>
              <button className="btn btn-ghost" onClick={handleCopy}>
                {copied ? "✓ 已复制" : "复制"}
              </button>
              <button className="btn btn-primary" onClick={handleDownload}>
                下载 YAML
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
