import React, { useState, useCallback } from "react";
import {
  ParameterEntry,
  ParameterValue,
  getParameterType,
  getParameterValue,
  displayValue,
  isArrayType,
} from "../types";
import { ChangeIndicator } from "./ChangeIndicator";

interface ParameterEditorProps {
  entry: ParameterEntry;
  onStage: (name: string, value: ParameterValue) => void;
  onApply: (name: string) => void;
  onReset: (name: string) => void;
}

export function ParameterEditor({
  entry,
  onStage,
  onApply,
  onReset,
}: ParameterEditorProps) {
  const isReadOnly = entry.read_only;

  const renderEditor = () => {
    if (isReadOnly) {
      return <span className="param-value-readonly">{displayValue(entry.current_value)}</span>;
    }

    const type = getParameterType(entry.current_value);
    const val = getParameterValue(entry.current_value);

    switch (type) {
      case "bool":
        return (
          <label className="bool-editor">
            <input
              type="checkbox"
              checked={!!val}
              onChange={(e) => {
                onStage(entry.name, { bool: e.target.checked });
              }}
            />
            <span className="checkbox-label">{val ? "true" : "false"}</span>
          </label>
        );

      case "integer": {
        const range = entry.integer_ranges[0];
        if (range) {
          return (
            <div className="slider-editor">
              <input
                type="range"
                min={range.from}
                max={range.to}
                step={range.step || 1}
                value={val ?? 0}
                onChange={(e) => {
                  onStage(entry.name, { integer: parseInt(e.target.value, 10) });
                }}
              />
              <input
                type="number"
                className="param-spinbox"
                min={range.from}
                max={range.to}
                step={range.step || 1}
                value={val ?? 0}
                onChange={(e) => {
                  onStage(entry.name, { integer: parseInt(e.target.value, 10) || 0 });
                }}
              />
            </div>
          );
        }
        return (
          <input
            type="number"
            className="param-spinbox"
            value={val ?? 0}
            onChange={(e) => {
              onStage(entry.name, { integer: parseInt(e.target.value, 10) || 0 });
            }}
          />
        );
      }

      case "double": {
        const range = entry.floating_ranges[0];
        if (range) {
          return (
            <div className="slider-editor">
              <input
                type="range"
                min={range.from}
                max={range.to}
                step={range.step || 0.01}
                value={val ?? 0.0}
                onChange={(e) => {
                  onStage(entry.name, { double: parseFloat(e.target.value) });
                }}
              />
              <input
                type="number"
                className="param-spinbox"
                min={range.from}
                max={range.to}
                step={range.step || 0.01}
                value={val ?? 0.0}
                onChange={(e) => {
                  onStage(entry.name, { double: parseFloat(e.target.value) || 0.0 });
                }}
              />
            </div>
          );
        }
        return (
          <input
            type="number"
            className="param-spinbox"
            step="any"
            value={val ?? 0.0}
            onChange={(e) => {
              onStage(entry.name, { double: parseFloat(e.target.value) || 0.0 });
            }}
          />
        );
      }

      case "string": {
        // Check if it looks like an enum (constraints mention specific values)
        if (entry.additional_constraints.includes("valid values")) {
          // Could render a dropdown — for now use text input
        }
        return (
          <input
            type="text"
            className="param-text"
            value={val ?? ""}
            onChange={(e) => {
              onStage(entry.name, { string: e.target.value });
            }}
          />
        );
      }

      case "byte_array":
      case "bool_array":
      case "integer_array":
      case "double_array":
      case "string_array":
        return (
          <span className="param-value-array">
            {displayValue(entry.current_value)}
          </span>
        );

      default:
        return <span className="param-value-unknown">{displayValue(entry.current_value)}</span>;
    }
  };

  return (
    <div className={`parameter-editor ${entry.changed ? "changed" : ""}`}>
      <div className="param-header">
        <span className="param-name" title={entry.name}>
          {entry.name}
        </span>
        <span className="param-type">{getParameterType(entry.current_value)}</span>
        {entry.read_only && <span className="param-badge badge-readonly">只读</span>}
        <ChangeIndicator changed={entry.changed} applyState={entry.apply_state} />
      </div>
      {entry.description && (
        <div className="param-description">{entry.description}</div>
      )}
      {entry.additional_constraints && (
        <div className="param-constraints">{entry.additional_constraints}</div>
      )}
      <div className="param-controls">
        {renderEditor()}
        <div className="param-actions">
          {entry.changed && !isReadOnly && (
            <>
              <button
                className="btn btn-sm btn-primary"
                onClick={() => onApply(entry.name)}
                title="提交到 ROS 节点"
              >
                应用
              </button>
              <button
                className="btn btn-sm btn-ghost"
                onClick={() => onReset(entry.name)}
                title="重置为原始值"
              >
                撤销
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
