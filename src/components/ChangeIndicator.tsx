import React from "react";
import { ApplyState } from "../types";

interface ChangeIndicatorProps {
  changed: boolean;
  applyState: ApplyState;
}

export function ChangeIndicator({ changed, applyState }: ChangeIndicatorProps) {
  if (!changed && applyState === "Unchanged") {
    return null;
  }

  let label: string;
  let className: string;

  if (applyState === "Pending") {
    label = "已修改（待提交）";
    className = "change-pending";
  } else if (applyState === "Applied") {
    label = "✓ 已提交";
    className = "change-applied";
  } else if (typeof applyState === "object" && "Failed" in applyState) {
    label = `✗ ${applyState.Failed}`;
    className = "change-failed";
  } else if (applyState === "Conflict") {
    label = "⚠ 外部冲突";
    className = "change-conflict";
  } else if (changed) {
    label = "已修改";
    className = "change-pending";
  } else {
    return null;
  }

  return (
    <span className={`change-indicator ${className}`} title={label}>
      {label}
    </span>
  );
}
