use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use crate::router::RoutingTableEntry;
use crate::network::NetworkTopology;

#[derive(Debug, Clone)]
struct DijkstraNode {
    router_id: u32,
    cost: u32,
    next_hop: Option<u32>,
}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DijkstraNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for DijkstraNode {}

pub struct SPFCalculator;

impl SPFCalculator {
    pub fn calculate_routes(
        topology: &NetworkTopology,
        source_router_id: u32,
    ) -> HashMap<u32, RoutingTableEntry> {
        let mut distances: HashMap<u32, u32> = HashMap::new();
        let mut next_hops: HashMap<u32, u32> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut routing_table = HashMap::new();

        // Initialize with source
        distances.insert(source_router_id, 0);
        heap.push(DijkstraNode {
            router_id: source_router_id,
            cost: 0,
            next_hop: None,
        });

        while let Some(node) = heap.pop() {
            // Skip if we've found a better path
            if let Some(&current_dist) = distances.get(&node.router_id) {
                if node.cost > current_dist {
                    continue;
                }
            }

            // Get all neighbors
            for link in topology.links.values() {
                let (neighbor_id, cost) = if link.router1_id == node.router_id {
                    (link.router2_id, link.cost)
                } else if link.router2_id == node.router_id {
                    (link.router1_id, link.cost)
                } else {
                    continue;
                };

                let new_cost = node.cost + cost;

                // Check if this is a better path
                if !distances.contains_key(&neighbor_id) || new_cost < distances[&neighbor_id] {
                    distances.insert(neighbor_id, new_cost);

                    // Determine next hop
                    let next_hop = if node.router_id == source_router_id {
                        neighbor_id
                    } else {
                        node.next_hop.unwrap_or(neighbor_id)
                    };

                    next_hops.insert(neighbor_id, next_hop);

                    heap.push(DijkstraNode {
                        router_id: neighbor_id,
                        cost: new_cost,
                        next_hop: Some(next_hop),
                    });
                }
            }
        }

        // Build routing table entries
        for (dest_router_id, &next_hop) in &next_hops {
            if *dest_router_id == source_router_id {
                continue;
            }

            // Find the interface to reach next hop
            let interface_info = topology.links.values()
                .find(|link| {
                    (link.router1_id == source_router_id && link.router2_id == next_hop) ||
                    (link.router2_id == source_router_id && link.router1_id == next_hop)
                })
                .and_then(|link| {
                    if link.router1_id == source_router_id {
                        topology.routers.get(&source_router_id)
                            .and_then(|r| r.interfaces.get(&link.router1_interface_id))
                    } else {
                        topology.routers.get(&source_router_id)
                            .and_then(|r| r.interfaces.get(&link.router2_interface_id))
                    }
                });

            if let Some(interface) = interface_info {
                let entry = RoutingTableEntry {
                    destination: format!("{}.{}.{}.{}", 1, 1, 1, dest_router_id),
                    netmask: "255.255.255.255".to_string(),
                    next_hop: format!("{}.{}.{}.{}", 1, 1, 1, next_hop),
                    interface_id: interface.id,
                    metric: distances[dest_router_id],
                    protocol: crate::router::RoutingProtocol::OSPF,
                };

                routing_table.insert(*dest_router_id, entry);
            }
        }

        routing_table
    }

    pub fn get_network_graph_json(topology: &NetworkTopology) -> String {
        let mut graph = serde_json::json!({
            "nodes": [],
            "edges": []
        });

        // Add nodes
        for (id, router) in &topology.routers {
            graph["nodes"].as_array_mut().unwrap().push(serde_json::json!({
                "id": id,
                "name": router.name,
                "ospf_enabled": router.ospf_state.is_some()
            }));
        }

        // Add edges
        for link in topology.links.values() {
            graph["edges"].as_array_mut().unwrap().push(serde_json::json!({
                "source": link.router1_id,
                "target": link.router2_id,
                "cost": link.cost
            }));
        }

        graph.to_string()
    }
}