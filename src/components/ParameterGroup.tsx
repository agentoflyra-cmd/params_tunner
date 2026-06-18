import React, { useState } from "react";
import { ParameterEntry, ParameterGroup as PG, buildParameterTree } from "../types";
import { ParameterEditor } from "./ParameterEditor";

interface ParameterGroupViewProps {
  parameters: ParameterEntry[];
  onStage: (name: string, value: any) => void;
  onApply: (name: string) => void;
  onReset: (name: string) => void;
}

export function ParameterGroupView({
  parameters,
  onStage,
  onApply,
  onReset,
}: ParameterGroupViewProps) {
  const groups = buildParameterTree(parameters);

  return (
    <div className="parameter-groups">
      {groups.map((group) => (
        <GroupSection
          key={group.prefix}
          group={group}
          onStage={onStage}
          onApply={onApply}
          onReset={onReset}
        />
      ))}
    </div>
  );
}

interface GroupSectionProps {
  group: PG;
  onStage: (name: string, value: any) => void;
  onApply: (name: string) => void;
  onReset: (name: string) => void;
}

function GroupSection({ group, onStage, onApply, onReset }: GroupSectionProps) {
  const [collapsed, setCollapsed] = useState(false);

  const hasChanges = group.children.some(
    (c) => c.type === "param" && c.entry.changed
  );

  return (
    <div className={`param-group ${hasChanges ? "has-changes" : ""}`}>
      <div
        className="param-group-header"
        onClick={() => setCollapsed(!collapsed)}
      >
        <span className="group-toggle">{collapsed ? "▶" : "▼"}</span>
        <span className="group-name">{group.prefix}</span>
        {hasChanges && <span className="group-badge">已修改</span>}
      </div>
      {!collapsed && (
        <div className="param-group-children">
          {group.children.map((child) => {
            if (child.type === "param") {
              return (
                <ParameterEditor
                  key={child.entry.name}
                  entry={child.entry}
                  onStage={onStage}
                  onApply={onApply}
                  onReset={onReset}
                />
              );
            }
            return null;
          })}
        </div>
      )}
    </div>
  );
}
