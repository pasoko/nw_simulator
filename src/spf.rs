use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use crate::router::{RoutingTableEntry, LSA, LSAData};
use crate::network::NetworkTopology;
use crate::console_log;

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
    pub fn calculate_routes_from_lsa(
        lsa_database: &HashMap<String, LSA>,
        source_router_id: u32,
        topology: &NetworkTopology,  // Still needed for interface info
    ) -> HashMap<u32, RoutingTableEntry> {
        let mut adjacencies: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();  // router_id -> [(neighbor_id, cost)]
        
        // Build adjacency graph from Router LSAs
        console_log!("SPF: Building adjacency graph from {} LSAs", lsa_database.len());
        for lsa in lsa_database.values() {
            if let LSAData::Router(router_lsa) = &lsa.data {
                let advertising_router = lsa.header.advertising_router
                    .split('.')
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                
                console_log!("SPF: Processing Router LSA from router {} with {} links", 
                    advertising_router, router_lsa.links.len());
                
                let mut neighbors = Vec::new();
                
                for link in &router_lsa.links {
                    // Extract neighbor router ID from link_id (format: "1.1.1.X")
                    if let Some(neighbor_id) = link.link_id
                        .split('.')
                        .last()
                        .and_then(|s| s.parse::<u32>().ok()) {
                        
                        if neighbor_id != advertising_router {
                            neighbors.push((neighbor_id, link.metric as u32));
                            console_log!("SPF:   Added adjacency: {} -> {} (metric {})", 
                                advertising_router, neighbor_id, link.metric);
                        }
                    }
                }
                
                adjacencies.insert(advertising_router, neighbors);
            }
        }
        
        // If no LSAs exist, return empty routing table
        if adjacencies.is_empty() {
            console_log!("SPF: No adjacencies found, returning empty routing table");
            return HashMap::new();
        }
        
        console_log!("SPF: Found adjacencies for {} routers", adjacencies.len());
        
        // Run Dijkstra on the LSA-derived graph
        let mut distances: HashMap<u32, u32> = HashMap::new();
        let mut next_hops: HashMap<u32, u32> = HashMap::new();
        let mut heap = BinaryHeap::new();
        let mut routing_table = HashMap::new();
        
        distances.insert(source_router_id, 0);
        heap.push(DijkstraNode {
            router_id: source_router_id,
            cost: 0,
            next_hop: None,
        });
        
        while let Some(node) = heap.pop() {
            if let Some(&current_dist) = distances.get(&node.router_id) {
                if node.cost > current_dist {
                    continue;
                }
            }
            
            // Get neighbors from adjacency list
            if let Some(neighbors) = adjacencies.get(&node.router_id) {
                for &(neighbor_id, cost) in neighbors {
                    let new_cost = node.cost + cost;
                    
                    if !distances.contains_key(&neighbor_id) || new_cost < distances[&neighbor_id] {
                        distances.insert(neighbor_id, new_cost);
                        
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
        }
        
        console_log!("SPF: Dijkstra complete, found paths to {} routers", next_hops.len());
        
        // Build routing table entries using topology for interface info
        for (dest_router_id, &next_hop) in &next_hops {
            if *dest_router_id == source_router_id {
                continue;
            }
            
            console_log!("SPF: Building route to router {} via next hop {}", dest_router_id, next_hop);
            
            // Find interface from topology
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
                
                console_log!("SPF:   Route entry: {} via {} on interface {} (metric {})", 
                    entry.destination, entry.next_hop, entry.interface_id, entry.metric);
                
                routing_table.insert(*dest_router_id, entry);
            } else {
                console_log!("SPF:   WARNING: No interface found for route to {}", dest_router_id);
            }
        }
        
        routing_table
    }
    
    // Keep old method for compatibility but have it use empty LSA database
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

}