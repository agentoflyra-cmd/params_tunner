#!/usr/bin/env python3
"""
ROS 2 Parameter Agent — stdin/stdout JSON-RPC sidecar.

Communicates with the Tauri Rust backend via JSON-RPC 2.0 over stdin/stdout.
Each line is a complete JSON-RPC request or response.

Supported methods:
    discover_targets        -> List of node summaries
    inspect_target          -> Full parameter details for one node
    set_parameter           -> Set a single parameter
    set_parameters          -> Batch set parameters
    shutdown                -> Graceful shutdown
"""

import sys
import json
import traceback
import threading
import rclpy
from rclpy.executors import MultiThreadedExecutor

from node_discovery import NodeDiscovery
from parameter_client import ParameterClient, ParameterValueMapper


class Ros2ParamAgent:
    """Main agent — manages ROS 2 nodes and handles JSON-RPC requests."""

    def __init__(self):
        rclpy.init(args=sys.argv[1:] if len(sys.argv) > 1 else [])
        self._discovery = NodeDiscovery()
        self._param_client = ParameterClient()

        self._executor = MultiThreadedExecutor()
        self._executor.add_node(self._discovery)
        self._executor.add_node(self._param_client)

        self._running = True
        self._spin_thread = threading.Thread(target=self._spin, daemon=True)
        self._spin_thread.start()

    def _spin(self):
        """Background spin the ROS 2 executor."""
        while self._running:
            self._executor.spin_once(timeout_sec=0.1)

    def handle_request(self, request: dict) -> dict:
        """Dispatch a JSON-RPC request and return the response."""
        method = request.get("method", "")
        params = request.get("params", {})
        req_id = request.get("id")

        try:
            if method == "discover_targets":
                result = self._handle_discover()
            elif method == "inspect_target":
                result = self._handle_inspect(params)
            elif method == "set_parameter":
                result = self._handle_set_parameter(params)
            elif method == "set_parameters":
                result = self._handle_set_parameters(params)
            elif method == "shutdown":
                result = self._handle_shutdown()
            else:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32601, "message": f"Method '{method}' not found"},
                }

            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": result,
            }

        except Exception as e:
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {"code": -1, "message": f"{type(e).__name__}: {str(e)}"},
            }

    def _handle_discover(self) -> list:
        """Handle discover_targets."""
        nodes = self._discovery.discover_with_details()
        # Format as ParameterTargetSummary list
        summaries = []
        for n in nodes:
            summaries.append({
                "full_node_name": n["full_node_name"],
                "namespace": n["namespace"],
                "node_name": n["node_name"],
                "state": n["state"],
                "parameter_count": 0,  # Will be filled on inspect
            })

        # Try to get parameter counts for available nodes
        for s in summaries:
            if s["state"] == "Available":
                names = self._param_client.list_parameters(s["full_node_name"])
                if names is not None:
                    s["parameter_count"] = len(names)

        return summaries

    def _handle_inspect(self, params: dict) -> dict:
        """Handle inspect_target — full parameter inspection."""
        node_name = params.get("node", "")
        if not node_name:
            raise ValueError("Missing 'node' parameter")

        # Parse node name into namespace + name
        if "/" in node_name.rstrip("/"):
            namespace, _, name = node_name.rstrip("/").rpartition("/")
            if namespace == "":
                namespace = "/"
        else:
            namespace = "/"
            name = node_name

        result = self._param_client.inspect_node_parameters(node_name)
        if result is None:
            # Node might have disappeared or timed out
            return {
                "full_node_name": node_name,
                "namespace": namespace,
                "node_name": name,
                "state": "Timeout",
                "parameters": [],
            }

        return {
            "full_node_name": node_name,
            "namespace": namespace,
            "node_name": name,
            "state": "Available",
            "parameters": result["parameters"],
        }

    def _handle_set_parameter(self, params: dict) -> dict:
        """Handle set_parameter."""
        node_name = params.get("node", "")
        param_name = params.get("name", "")
        param_value = params.get("value", {})

        if not node_name or not param_name:
            raise ValueError("Missing 'node' or 'name' parameter")

        param = ParameterValueMapper.from_json(param_name, param_value)
        results = self._param_client.set_parameters(node_name, [param])

        if results is None:
            return {
                "name": param_name,
                "successful": False,
                "reason": "Service call failed or timed out",
            }

        result = results[0]
        return {
            "name": param_name,
            "successful": result.successful,
            "reason": result.reason if hasattr(result, 'reason') and not result.successful else None,
        }

    def _handle_set_parameters(self, params: dict) -> list:
        """Handle set_parameters (batch)."""
        node_name = params.get("node", "")
        raw_params = params.get("parameters", [])
        atomic = params.get("atomic", False)

        if not node_name:
            raise ValueError("Missing 'node' parameter")

        ros_params = []
        for p in raw_params:
            ros_params.append(
                ParameterValueMapper.from_json(p["name"], p["value"])
            )

        if atomic:
            result = self._param_client.set_parameters_atomically(node_name, ros_params)
            if result is None:
                return [
                    {"name": p.name, "successful": False, "reason": "Service call failed"}
                    for p in ros_params
                ]
            # Atomic: all-or-nothing
            successful = result.successful
            reason = result.reason if hasattr(result, 'reason') and not successful else None
            return [
                {"name": p.name, "successful": successful, "reason": reason}
                for p in ros_params
            ]
        else:
            results = self._param_client.set_parameters(node_name, ros_params)
            if results is None:
                return [
                    {"name": p.name, "successful": False, "reason": "Service call failed"}
                    for p in ros_params
                ]
            return [
                {
                    "name": ros_params[i].name,
                    "successful": r.successful,
                    "reason": r.reason if hasattr(r, 'reason') and not r.successful else None,
                }
                for i, r in enumerate(results)
            ]

    def _handle_shutdown(self) -> dict:
        """Handle shutdown."""
        self._running = False
        return {"status": "shutting_down"}

    def shutdown(self):
        """Clean shutdown."""
        self._running = False
        if self._spin_thread.is_alive():
            self._spin_thread.join(timeout=2.0)
        self._param_client.destroy_node()
        self._discovery.destroy_node()
        rclpy.shutdown()


def main():
    """Main entry point — reads JSON-RPC requests from stdin, writes responses to stdout."""
    agent = Ros2ParamAgent()

    try:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except json.JSONDecodeError as e:
                response = {
                    "jsonrpc": "2.0",
                    "id": None,
                    "error": {"code": -32700, "message": f"Parse error: {e}"},
                }
                print(json.dumps(response), flush=True)
                continue

            response = agent.handle_request(request)
            print(json.dumps(response), flush=True)

            # Check for shutdown
            if request.get("method") == "shutdown":
                break

    except (BrokenPipeError, EOFError):
        pass
    finally:
        agent.shutdown()


if __name__ == "__main__":
    main()
