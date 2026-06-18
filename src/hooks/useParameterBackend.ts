import { useState, useCallback, useRef, useEffect } from "react";
import {
  ParameterTargetSummary,
  ParameterTarget,
  ParameterEntry,
  ApplyResult,
  ExportSelection,
} from "../types";

// Detect if running inside Tauri
const isTauri = typeof window !== "undefined" && "__TAURI__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import("@tauri-apps/api/tauri");
    return invoke<T>(cmd, args);
  }
  throw new Error("Not running in Tauri");
}

export interface BackendState {
  loading: boolean;
  error: string | null;
  targets: ParameterTargetSummary[];
  currentTarget: ParameterTarget | null;
  modifiedCount: number;
}

export function useParameterBackend() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [targets, setTargets] = useState<ParameterTargetSummary[]>([]);
  const [currentTarget, setCurrentTarget] = useState<ParameterTarget | null>(null);
  const [targetCache, setTargetCache] = useState<Map<string, ParameterTarget>>(new Map());
  const cacheRef = useRef(targetCache);
  cacheRef.current = targetCache;

  // Discover nodes
  const discover = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await tauriInvoke<ParameterTargetSummary[]>("discover_targets");
      setTargets(result);
      // Clear cache on rediscovery
      setTargetCache(new Map());
      setCurrentTarget(null);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Inspect a node
  const inspect = useCallback(async (fullNodeName: string) => {
    // Check cache first
    if (cacheRef.current.has(fullNodeName)) {
      const cached = cacheRef.current.get(fullNodeName)!;
      setCurrentTarget(cached);
      return cached;
    }

    setLoading(true);
    setError(null);
    try {
      const result = await tauriInvoke<ParameterTarget>("inspect_target", {
        full_node_name: fullNodeName,
      });
      setTargetCache((prev) => {
        const next = new Map(prev);
        next.set(fullNodeName, result);
        return next;
      });
      setCurrentTarget(result);
      return result;
    } catch (e: any) {
      setError(String(e));
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  // Refresh current target
  const refreshCurrent = useCallback(async () => {
    if (!currentTarget) return;
    await inspect(currentTarget.full_node_name);
  }, [currentTarget, inspect]);

  // Set a single parameter (local staging only — doesn't send to ROS)
  const stageParameter = useCallback(
    (paramName: string, value: any) => {
      if (!currentTarget) return;
      setCurrentTarget((prev) => {
        if (!prev) return prev;
        const params = prev.parameters.map((p) => {
          if (p.name !== paramName) return p;
          return {
            ...p,
            current_value: value as any,
            changed: JSON.stringify(value) !== JSON.stringify(p.original_value),
            apply_state:
              JSON.stringify(value) !== JSON.stringify(p.original_value)
                ? "Pending"
                : ("Unchanged" as any),
          };
        });
        return { ...prev, parameters: params };
      });
      // Update cache too
      setTargetCache((prev) => {
        const next = new Map(prev);
        const existing = next.get(currentTarget.full_node_name);
        if (existing) {
          const params = existing.parameters.map((p) => {
            if (p.name !== paramName) return p;
            return {
              ...p,
              current_value: value as any,
              changed: JSON.stringify(value) !== JSON.stringify(p.original_value),
              apply_state:
                JSON.stringify(value) !== JSON.stringify(p.original_value)
                  ? "Pending"
                  : ("Unchanged" as any),
            };
          });
          next.set(currentTarget.full_node_name, { ...existing, parameters: params });
        }
        return next;
      });
    },
    [currentTarget]
  );

  // Submit a parameter to ROS
  const applyParameter = useCallback(
    async (node: string, name: string): Promise<ApplyResult | null> => {
      setError(null);
      try {
        const entry = currentTarget?.parameters.find((p) => p.name === name);
        if (!entry) return null;

        const result = await tauriInvoke<ApplyResult>("set_parameter", {
          node,
          name,
          value: entry.current_value,
        });

        // Update apply state
        setCurrentTarget((prev) => {
          if (!prev) return prev;
          const params = prev.parameters.map((p) => {
            if (p.name !== name) return p;
            return {
              ...p,
              apply_state: result.successful
                ? ("Applied" as any)
                : ({ Failed: result.reason || "Unknown error" } as any),
              changed: !result.successful ? p.changed : false,
              original_value: result.successful
                ? p.current_value
                : p.original_value,
            };
          });
          return { ...prev, parameters: params };
        });

        return result;
      } catch (e: any) {
        setError(String(e));
        return null;
      }
    },
    [currentTarget]
  );

  // Submit all pending parameters
  const applyAllPending = useCallback(async (): Promise<ApplyResult[]> => {
    if (!currentTarget) return [];
    setError(null);

    const pending = currentTarget.parameters.filter(
      (p) => p.apply_state === "Pending"
    );
    if (pending.length === 0) return [];

    try {
      const parameters = pending.map((p) => ({
        name: p.name,
        value: p.current_value,
      }));

      const results = await tauriInvoke<ApplyResult[]>("set_parameters", {
        node: currentTarget.full_node_name,
        parameters,
        atomic: false,
      });

      // Update apply states
      setCurrentTarget((prev) => {
        if (!prev) return prev;
        const resultMap = new Map(results.map((r) => [r.name, r]));
        const params = prev.parameters.map((p) => {
          const r = resultMap.get(p.name);
          if (!r) return p;
          return {
            ...p,
            apply_state: r.successful
              ? ("Applied" as any)
              : ({ Failed: r.reason || "Unknown error" } as any),
            changed: !r.successful ? p.changed : false,
            original_value: r.successful ? p.current_value : p.original_value,
          };
        });
        return { ...prev, parameters: params };
      });

      return results;
    } catch (e: any) {
      setError(String(e));
      return [];
    }
  }, [currentTarget]);

  // Reset a parameter to original value
  const resetParameter = useCallback(
    (paramName: string) => {
      if (!currentTarget) return;
      const entry = currentTarget.parameters.find((p) => p.name === paramName);
      if (!entry) return;

      setCurrentTarget((prev) => {
        if (!prev) return prev;
        const params = prev.parameters.map((p) => {
          if (p.name !== paramName) return p;
          return {
            ...p,
            current_value: p.original_value,
            changed: false,
            apply_state: "Unchanged" as any,
          };
        });
        return { ...prev, parameters: params };
      });
    },
    [currentTarget]
  );

  // Export YAML
  const exportYaml = useCallback(
    async (selection: ExportSelection): Promise<string> => {
      setError(null);
      try {
        return await tauriInvoke<string>("export_yaml", { selection });
      } catch (e: any) {
        setError(String(e));
        return "";
      }
    },
    []
  );

  // Compute modified count across all cached targets
  const getModifiedCount = useCallback((): number => {
    let count = 0;
    for (const target of cacheRef.current.values()) {
      count += target.parameters.filter((p) => p.changed).length;
    }
    return count;
  }, []);

  return {
    loading,
    error,
    targets,
    currentTarget,
    discover,
    inspect,
    refreshCurrent,
    stageParameter,
    applyParameter,
    applyAllPending,
    resetParameter,
    exportYaml,
    getModifiedCount,
    setError,
  };
}
