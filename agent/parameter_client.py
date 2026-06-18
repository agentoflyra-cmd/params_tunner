"""ROS 2 Parameter Client — read/write parameters via ROS 2 services."""

import rclpy
from rclpy.node import Node
from rclpy.parameter import Parameter
from rclpy.callback_groups import ReentrantCallbackGroup
from typing import List, Dict, Any, Optional, Tuple
import time


class ParameterValueMapper:
    """Maps between ROS 2 Parameter objects and JSON-serializable dicts."""

    @staticmethod
    def to_json(value: Parameter) -> Dict:
        """Convert a ROS 2 Parameter to a JSON-serializable dict."""
        ptype = value.type
        type_name = Parameter.Type.STRING if ptype == Parameter.Type.STRING else ptype
        # Use the string representation of the type enum
        type_map = {
            Parameter.Type.NOT_SET: "not_set",
            Parameter.Type.BOOL: "bool",
            Parameter.Type.INTEGER: "integer",
            Parameter.Type.DOUBLE: "double",
            Parameter.Type.STRING: "string",
            Parameter.Type.BYTE_ARRAY: "byte_array",
            Parameter.Type.BOOL_ARRAY: "bool_array",
            Parameter.Type.INTEGER_ARRAY: "integer_array",
            Parameter.Type.DOUBLE_ARRAY: "double_array",
            Parameter.Type.STRING_ARRAY: "string_array",
        }

        type_name = type_map.get(ptype, "not_set")

        if ptype == Parameter.Type.NOT_SET:
            val = None
        elif ptype == Parameter.Type.BOOL:
            val = value.value
        elif ptype == Parameter.Type.INTEGER:
            val = value.value
        elif ptype == Parameter.Type.DOUBLE:
            val = value.value
        elif ptype == Parameter.Type.STRING:
            val = value.value
        elif ptype == Parameter.Type.BYTE_ARRAY:
            val = list(value.value)
        elif ptype == Parameter.Type.BOOL_ARRAY:
            val = list(value.value)
        elif ptype == Parameter.Type.INTEGER_ARRAY:
            val = list(value.value)
        elif ptype == Parameter.Type.DOUBLE_ARRAY:
            val = list(value.value)
        elif ptype == Parameter.Type.STRING_ARRAY:
            val = list(value.value)
        else:
            val = None

        return {type_name: val}

    @staticmethod
    def from_json(name: str, json_val: Dict) -> Parameter:
        """Convert a JSON value dict back to a ROS 2 Parameter."""
        # json_val is like {"double": 0.55} or {"integer": 42}
        for type_name, value in json_val.items():
            if type_name == "not_set" or value is None:
                return Parameter(name, Parameter.Type.NOT_SET, None)
            elif type_name == "bool":
                return Parameter(name, Parameter.Type.BOOL, value)
            elif type_name == "integer":
                return Parameter(name, Parameter.Type.INTEGER, value)
            elif type_name == "double":
                return Parameter(name, Parameter.Type.DOUBLE, float(value))
            elif type_name == "string":
                return Parameter(name, Parameter.Type.STRING, str(value))
            elif type_name == "byte_array":
                return Parameter(name, Parameter.Type.BYTE_ARRAY, bytes(value))
            elif type_name == "bool_array":
                return Parameter(name, Parameter.Type.BOOL_ARRAY, value)
            elif type_name == "integer_array":
                return Parameter(name, Parameter.Type.INTEGER_ARRAY, value)
            elif type_name == "double_array":
                return Parameter(name, Parameter.Type.DOUBLE_ARRAY, value)
            elif type_name == "string_array":
                return Parameter(name, Parameter.Type.STRING_ARRAY, value)

        return Parameter(name, Parameter.Type.NOT_SET, None)


class ParameterClient(Node):
    """High-level wrapper around rclpy AsyncParametersClient."""

    def __init__(self):
        super().__init__("param_tuner_client")
        self._clients: Dict[str, Any] = {}
        self._callback_group = ReentrantCallbackGroup()

    def _get_client(self, node_name: str):
        """Get or create a parameter client for the given node."""
        if node_name not in self._clients:
            from rclpy.parameter_client import AsyncParametersClient
            self._clients[node_name] = AsyncParametersClient(
                self, node_name, callback_group=self._callback_group
            )
        return self._clients[node_name]

    def list_parameters(self, node_name: str, timeout: float = 3.0) -> Optional[List[str]]:
        """List all parameter names for a node."""
        client = self._get_client(node_name)
        try:
            future = client.list_parameters([], timeout_sec=timeout)
            rclpy.spin_until_future_complete(self, future, timeout_sec=timeout)
            if future.result() is not None:
                return list(future.result().result.names)
        except Exception as e:
            self.get_logger().warn(f"Failed to list parameters for {node_name}: {e}")
        return None

    def get_parameters(self, node_name: str, names: List[str],
                       timeout: float = 3.0) -> Optional[List[Parameter]]:
        """Get current values of specific parameters."""
        client = self._get_client(node_name)
        try:
            future = client.get_parameters(names, timeout_sec=timeout)
            rclpy.spin_until_future_complete(self, future, timeout_sec=timeout)
            if future.result() is not None:
                return list(future.result())
        except Exception as e:
            self.get_logger().warn(f"Failed to get parameters for {node_name}: {e}")
        return None

    def describe_parameters(self, node_name: str, names: List[str],
                            timeout: float = 3.0) -> Optional[List]:
        """Get parameter descriptors (type, description, constraints)."""
        client = self._get_client(node_name)
        try:
            future = client.describe_parameters(names, timeout_sec=timeout)
            rclpy.spin_until_future_complete(self, future, timeout_sec=timeout)
            if future.result() is not None:
                return list(future.result())
        except Exception as e:
            self.get_logger().warn(f"Failed to describe parameters for {node_name}: {e}")
        return None

    def set_parameters(self, node_name: str, params: List[Parameter],
                       timeout: float = 3.0) -> Optional[List]:
        """Set parameters on a node. Returns list of result objects."""
        client = self._get_client(node_name)
        try:
            future = client.set_parameters(params, timeout_sec=timeout)
            rclpy.spin_until_future_complete(self, future, timeout_sec=timeout)
            if future.result() is not None:
                return list(future.result())
        except Exception as e:
            self.get_logger().warn(f"Failed to set parameters for {node_name}: {e}")
        return None

    def set_parameters_atomically(self, node_name: str, params: List[Parameter],
                                  timeout: float = 3.0) -> Optional:
        """Set parameters atomically. Returns a single result with overall success."""
        client = self._get_client(node_name)
        try:
            future = client.set_parameters_atomically(params, timeout_sec=timeout)
            rclpy.spin_until_future_complete(self, future, timeout_sec=timeout)
            if future.result() is not None:
                return future.result()
        except Exception as e:
            self.get_logger().warn(f"Failed to set parameters atomically for {node_name}: {e}")
        return None

    def inspect_node_parameters(self, node_name: str) -> Optional[Dict]:
        """
        Full parameter inspection: list -> get -> describe.

        Returns:
            Dict with 'parameters' list, or None on failure.
        """
        names = self.list_parameters(node_name)
        if names is None:
            return None

        values = self.get_parameters(node_name, names)
        descriptors = self.describe_parameters(node_name, names)

        if values is None:
            return None

        param_list = []
        for i, name in enumerate(names):
            value = values[i] if i < len(values) else None
            desc = descriptors[i] if descriptors and i < len(descriptors) else None

            entry = {
                "name": name,
            }

            if value is not None:
                entry["current_value"] = ParameterValueMapper.to_json(value)
                entry["original_value"] = ParameterValueMapper.to_json(value)
            else:
                entry["current_value"] = {"not_set": None}
                entry["original_value"] = {"not_set": None}

            if desc is not None:
                entry["description"] = desc.description or ""
                entry["additional_constraints"] = desc.additional_constraints or ""
                entry["read_only"] = desc.read_only
                entry["dynamic_typing"] = desc.dynamic_typing
                entry["integer_ranges"] = [
                    {"from": r.from_value, "to": r.to_value, "step": r.step}
                    for r in desc.integer_range
                ]
                entry["floating_ranges"] = [
                    {"from": r.from_value, "to": r.to_value, "step": r.step}
                    for r in desc.floating_point_range
                ]
            else:
                entry["description"] = ""
                entry["additional_constraints"] = ""
                entry["read_only"] = False
                entry["dynamic_typing"] = False
                entry["integer_ranges"] = []
                entry["floating_ranges"] = []

            entry["changed"] = False
            entry["apply_state"] = "Unchanged"

            param_list.append(entry)

        return {"parameters": param_list}
