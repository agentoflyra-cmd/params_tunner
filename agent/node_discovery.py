"""ROS 2 node discovery — find nodes and check parameter service availability."""

import rclpy
from rclpy.node import Node
from rclpy.parameter import Parameter
from typing import List, Dict, Optional, Tuple
import asyncio
import time


class NodeDiscovery(Node):
    """Discovers ROS 2 nodes and checks their parameter service availability."""

    def __init__(self):
        super().__init__("param_tuner_discovery")

    def get_all_nodes(self) -> List[Tuple[str, str]]:
        """
        Discover all nodes in the ROS graph.

        Returns:
            List of (node_name, namespace) tuples.
        """
        node_names_and_ns = (
            self.get_node_graph_interface()
            .get_node_names_and_namespaces()
        )
        result = []
        for name, ns in node_names_and_ns:
            # Skip self
            if name == "param_tuner_discovery":
                continue
            result.append((name, ns))
        return result

    def check_parameter_service(self, node_name: str, node_namespace: str) -> bool:
        """
        Check if a node provides the list_parameters service.

        Args:
            node_name: Base node name (e.g., "controller_server")
            node_namespace: Node namespace (e.g., "/navigation")

        Returns:
            True if the parameter service exists.
        """
        full_name = f"{node_namespace}/{node_name}" if node_namespace != "/" else f"/{node_name}"
        full_name = full_name.replace("//", "/")

        services = self.get_service_names_and_types()
        list_svc = f"{full_name}/list_parameters"
        svc_types = dict(services)

        if list_svc in svc_types:
            return True
        return False

    def discover_with_details(self) -> List[Dict]:
        """
        Discover all nodes and check their parameter status.

        Returns:
            List of dicts with keys: full_node_name, namespace, node_name, has_parameter_service
        """
        nodes = self.get_all_nodes()
        results = []

        for name, ns in nodes:
            full_name = f"{ns}/{name}" if ns != "/" else f"/{name}"
            full_name = full_name.replace("//", "/")

            has_svc = self.check_parameter_service(name, ns)

            results.append({
                "full_node_name": full_name,
                "namespace": ns if ns else "/",
                "node_name": name,
                "has_parameter_service": has_svc,
                "state": "Available" if has_svc else "NoParameterService",
            })

        return results
