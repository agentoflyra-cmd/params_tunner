use crate::parameter_model::{
    is_internal_parameter, ExportSelection, ParameterEntry, ParameterTarget, ParameterTargetState,
    ParameterValue,
};

/// 根据参数名推断 YAML 缩进层级
/// 如 "FollowPath.PathAlign.scale" → 键层级用点分隔
fn indent_for_name(name: &str, base_indent: usize) -> usize {
    // 对带点的多层参数名，每层额外缩进 2 空格
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    if parts.len() > 1 {
        base_indent
    } else {
        base_indent
    }
}

/// 将单个参数值格式化为 YAML 行
fn format_yaml_value(name: &str, value: &ParameterValue, indent: usize) -> String {
    let prefix = " ".repeat(indent);

    match value {
        ParameterValue::NotSet => format!("{} {}: null", prefix, name),
        ParameterValue::Bool(v) => format!("{} {}: {}", prefix, name, v),
        ParameterValue::Integer(v) => format!("{} {}: {}", prefix, name, v),
        ParameterValue::Double(v) => format!("{} {}: {}", prefix, name, format_yaml_float(v)),
        ParameterValue::String(v) => {
            if v.contains('\n') || v.contains(": ") || v.contains('#') {
                // 多行或特殊字符用引号或 block scalar
                if v.contains('\n') {
                    format!("{} {}: |\n{}", prefix, name, indent_each_line(v, indent + 2))
                } else {
                    format!("{} {}: '{}'", prefix, name, v.replace('\'', "''"))
                }
            } else if v.is_empty() {
                format!("{} {}: ''", prefix, name)
            } else {
                format!("{} {}: {}", prefix, name, v)
            }
        }
        ParameterValue::ByteArray(arr) => {
            let items: Vec<String> = arr.iter().map(|b| format!("{}", b)).collect();
            format!("{} {}: [{}]", prefix, name, items.join(", "))
        }
        ParameterValue::BoolArray(arr) => {
            let items: Vec<String> = arr.iter().map(|b| b.to_string()).collect();
            format!("{} {}: [{}]", prefix, name, items.join(", "))
        }
        ParameterValue::IntegerArray(arr) => {
            let items: Vec<String> = arr.iter().map(|i| i.to_string()).collect();
            format!("{} {}: [{}]", prefix, name, items.join(", "))
        }
        ParameterValue::DoubleArray(arr) => {
            let items: Vec<String> = arr.iter().map(|f| format_yaml_float(f)).collect();
            format!("{} {}: [{}]", prefix, name, items.join(", "))
        }
        ParameterValue::StringArray(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|s| {
                    if s.contains(' ') || s.contains(':') {
                        format!("'{}'", s.replace('\'', "''"))
                    } else {
                        s.clone()
                    }
                })
                .collect();
            format!("{} {}: [{}]", prefix, name, items.join(", "))
        }
    }
}

fn format_yaml_float(v: &f64) -> String {
    if v.is_infinite() {
        if *v > 0.0 {
            ".inf".to_string()
        } else {
            "-.inf".to_string()
        }
    } else if v.is_nan() {
        ".nan".to_string()
    } else if v.fract() == 0.0 && v.is_finite() {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

fn indent_each_line(text: &str, indent: usize) -> String {
    let prefix = " ".repeat(indent);
    text.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 分组参数名（按首个点前的前缀分组）
/// "FollowPath.max_vel_x" → 保持为平铺键（ROS 2 参数名本身就是点分隔）
/// 这里我们保持原样输出，按节点组织
fn filter_parameters<'a>(
    params: &'a [ParameterEntry],
    selection: &'a ExportSelection,
) -> Vec<&'a ParameterEntry> {
    params
        .iter()
        .filter(|p| {
            // 仅修改参数过滤
            if selection.only_changed && !p.changed {
                return false;
            }
            // 内部参数过滤
            if !selection.include_internal && is_internal_parameter(&p.name) {
                return false;
            }
            // 只读参数过滤
            if !selection.include_readonly && p.read_only {
                return false;
            }
            true
        })
        .collect()
}

/// 将单个节点导出为 YAML 字符串（标准 ROS 2 格式）
pub fn export_target_to_yaml(
    target: &ParameterTarget,
    selection: &ExportSelection,
) -> String {
    let filtered = filter_parameters(&target.parameters, selection);
    if filtered.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str(&format!("{}:\n", target.full_node_name));
    output.push_str("  ros__parameters:\n");

    for param in &filtered {
        let line = format_yaml_value(&param.name, &param.current_value, 4);
        output.push_str(&format!("  {}\n", line.trim_start()));
    }

    output.push('\n');
    output
}

/// 将多个节点导出为单个 YAML 文件字符串
pub fn export_all_to_yaml(targets: &[ParameterTarget], selection: &ExportSelection) -> String {
    let mut output = String::new();
    output.push_str("# ROS 2 Parameters — Exported by ROS 2 Param Tuner\n");
    output.push_str("# Generated: auto\n\n");

    for target in targets {
        if target.state != ParameterTargetState::Available {
            continue;
        }
        let section = export_target_to_yaml(target, selection);
        if !section.is_empty() {
            output.push_str(&section);
        }
    }

    output
}

/// 按 namespace 分组导出
pub fn export_by_namespace(
    targets: &[ParameterTarget],
    selection: &ExportSelection,
) -> Vec<(String, String)> {
    let mut ns_map: std::collections::BTreeMap<String, Vec<&ParameterTarget>> =
        std::collections::BTreeMap::new();

    for target in targets {
        if target.state != ParameterTargetState::Available {
            continue;
        }
        let ns = if target.namespace.is_empty() {
            "/".to_string()
        } else {
            target.namespace.clone()
        };
        ns_map.entry(ns).or_default().push(target);
    }

    ns_map
        .into_iter()
        .map(|(ns, targets_in_ns)| {
            let filename = if ns == "/" {
                "root.yaml".to_string()
            } else {
                // 去掉前导 / 并替换 / 为 _
                let clean = ns.trim_start_matches('/').replace('/', "_");
                if clean.is_empty() {
                    "root.yaml".to_string()
                } else {
                    format!("{}.yaml", clean)
                }
            };

            let mut yaml = String::new();
            yaml.push_str(&format!("# Namespace: {}\n\n", ns));
            for target in targets_in_ns {
                let section = export_target_to_yaml(target, selection);
                if !section.is_empty() {
                    yaml.push_str(&section);
                }
            }

            (filename, yaml)
        })
        .collect()
}

/// 按节点分文件导出
pub fn export_by_node(
    targets: &[ParameterTarget],
    selection: &ExportSelection,
) -> Vec<(String, String)> {
    targets
        .iter()
        .filter(|t| t.state == ParameterTargetState::Available)
        .map(|target| {
            let filename = format!("{}.yaml", target.full_node_name.trim_start_matches('/').replace('/', "_"));
            let yaml = export_target_to_yaml(target, selection);
            (filename, yaml)
        })
        .filter(|(_, yaml)| !yaml.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameter_model::{
        ApplyState, ParameterEntry, ParameterTarget, ParameterTargetState, ParameterValue,
    };

    fn sample_target(name: &str) -> ParameterTarget {
        ParameterTarget {
            full_node_name: name.to_string(),
            namespace: "/navigation".to_string(),
            node_name: name.trim_start_matches("/navigation/").to_string(),
            state: ParameterTargetState::Available,
            parameters: vec![
                ParameterEntry {
                    name: "controller_frequency".to_string(),
                    current_value: ParameterValue::Double(20.0),
                    original_value: ParameterValue::Double(20.0),
                    description: "Controller update frequency".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![FloatingPointRange {
                        from: 0.0,
                        to: 100.0,
                        step: 0.1,
                    }],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
                ParameterEntry {
                    name: "FollowPath.max_vel_x".to_string(),
                    current_value: ParameterValue::Double(0.55),
                    original_value: ParameterValue::Double(0.55),
                    description: "Max velocity in X".to_string(),
                    additional_constraints: String::new(),
                    read_only: false,
                    dynamic_typing: false,
                    integer_ranges: vec![],
                    floating_ranges: vec![],
                    changed: false,
                    apply_state: ApplyState::Unchanged,
                },
            ],
        }
    }

    #[test]
    fn test_single_target_export() {
        let target = sample_target("/navigation/controller_server");
        let selection = ExportSelection::default();
        let yaml = export_target_to_yaml(&target, &selection);
        assert!(yaml.contains("/navigation/controller_server:"));
        assert!(yaml.contains("ros__parameters:"));
        assert!(yaml.contains("controller_frequency: 20.0"));
    }

    #[test]
    fn test_only_changed_filter() {
        let mut target = sample_target("/navigation/controller_server");
        target.parameters[0].changed = true;
        target.parameters[1].changed = false;

        let selection = ExportSelection {
            only_changed: true,
            ..Default::default()
        };
        let yaml = export_target_to_yaml(&target, &selection);
        assert!(yaml.contains("controller_frequency"));
        assert!(!yaml.contains("FollowPath.max_vel_x"));
    }
}
