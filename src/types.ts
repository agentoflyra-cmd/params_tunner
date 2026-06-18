// --- ROS 2 Parameter Types (mirrors Rust parameter_model.rs) ---

export type ParameterTargetState =
  | "Available"
  | "NoParameterService"
  | "Timeout"
  | "LifecycleInactive"
  | "Disappeared";

export type ApplyState =
  | "Unchanged"
  | "Pending"
  | "Applied"
  | { Failed: string }
  | "Conflict";

export type ParameterValue =
  | { not_set: null }
  | { bool: boolean }
  | { integer: number }
  | { double: number }
  | { string: string }
  | { byte_array: number[] }
  | { bool_array: boolean[] }
  | { integer_array: number[] }
  | { double_array: number[] }
  | { string_array: string[] };

export interface IntegerRange {
  from: number;
  to: number;
  step: number;
}

export interface FloatingPointRange {
  from: number;
  to: number;
  step: number;
}

export interface ParameterEntry {
  name: string;
  current_value: ParameterValue;
  original_value: ParameterValue;
  description: string;
  additional_constraints: string;
  read_only: boolean;
  dynamic_typing: boolean;
  integer_ranges: IntegerRange[];
  floating_ranges: FloatingPointRange[];
  changed: boolean;
  apply_state: ApplyState;
}

export interface ParameterTargetSummary {
  full_node_name: string;
  namespace: string;
  node_name: string;
  state: ParameterTargetState;
  parameter_count: number;
}

export interface ParameterTarget {
  full_node_name: string;
  namespace: string;
  node_name: string;
  state: ParameterTargetState;
  parameters: ParameterEntry[];
}

export interface ApplyResult {
  name: string;
  successful: boolean;
  reason: string | null;
}

export interface ExportSelection {
  nodes?: string[];
  only_changed: boolean;
  include_internal: boolean;
  include_readonly: boolean;
  split_by_namespace: boolean;
  split_by_node: boolean;
  max_nodes_per_file?: number;
}

// --- Utility functions ---

export function getParameterType(value: ParameterValue): string {
  return Object.keys(value)[0] || "not_set";
}

export function getParameterValue(value: ParameterValue): any {
  const key = Object.keys(value)[0] as keyof ParameterValue;
  return (value as any)[key];
}

export function displayValue(value: ParameterValue): string {
  const key = Object.keys(value)[0];
  const val = (value as any)[key];

  switch (key) {
    case "not_set":
      return "<not set>";
    case "bool":
      return String(val);
    case "integer":
      return String(val);
    case "double":
      return typeof val === "number"
        ? Number.isInteger(val)
          ? val.toFixed(1)
          : String(val)
        : String(val);
    case "string":
      return val ?? "";
    case "byte_array":
      return `[${(val as number[]).length} bytes]`;
    case "bool_array":
      return `[${(val as boolean[]).join(", ")}]`;
    case "integer_array":
      return `[${(val as number[]).join(", ")}]`;
    case "double_array":
      return `[${(val as number[]).join(", ")}]`;
    case "string_array":
      return `[${(val as string[]).join(", ")}]`;
    default:
      return String(val);
  }
}

export function isArrayType(value: ParameterValue): boolean {
  const key = Object.keys(value)[0];
  return (
    key === "byte_array" ||
    key === "bool_array" ||
    key === "integer_array" ||
    key === "double_array" ||
    key === "string_array"
  );
}

export function isInternalParameter(name: string): boolean {
  return (
    name === "use_sim_time" ||
    name === "start_type_description_service" ||
    name.startsWith("qos_overrides.")
  );
}

/** Group parameters by dot-separated prefix for tree display */
export interface ParameterGroup {
  prefix: string;
  children: ParameterGroupItem[];
}

export type ParameterGroupItem =
  | { type: "param"; entry: ParameterEntry }
  | { type: "group"; group: ParameterGroup };

export function buildParameterTree(parameters: ParameterEntry[]): ParameterGroup[] {
  const groups = new Map<string, ParameterEntry[]>();
  const rootParams: ParameterEntry[] = [];

  for (const p of parameters) {
    const dotIndex = p.name.indexOf(".");
    if (dotIndex > 0) {
      const prefix = p.name.substring(0, dotIndex);
      if (!groups.has(prefix)) {
        groups.set(prefix, []);
      }
      groups.get(prefix)!.push(p);
    } else {
      rootParams.push(p);
    }
  }

  const result: ParameterGroup[] = [];
  for (const [prefix, entries] of groups) {
    const children: ParameterGroupItem[] = entries.map((e) => ({
      type: "param" as const,
      entry: e,
    }));
    result.push({
      prefix,
      children,
    });
  }

  // Root params at the end
  if (rootParams.length > 0) {
    result.push({
      prefix: "(其他)",
      children: rootParams.map((e) => ({ type: "param" as const, entry: e })),
    });
  }

  return result;
}
