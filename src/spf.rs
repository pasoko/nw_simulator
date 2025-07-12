use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use crate::router::{RoutingTableEntry, LSA, LSAData};
use crate::network::NetworkTopology;
use crate::console_log;

#[cfg(test)]
#[path = "spf_test.rs"]
mod spf_test;

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

/// SPF（Shortest Path First）アルゴリズムを実装するための計算機
/// 
/// このアルゴリズムは、OSPFプロトコルで使用されるダイクストラアルゴリズムを実装し、
/// 指定されたソースルーターから他の全てのルーターへの最短パスを計算します。
/// 
/// ## アルゴリズムの概要
/// 1. ソースルーターをコスト0で初期化
/// 2. プライオリティキュー（最小ヒープ）を使用して、コストが最小のノードを選択
/// 3. 選択されたノードの隣接ノードのコストを更新
/// 4. 全てのノードが処理されるまで繰り返し
/// 
/// ## 入力
/// - LSAデータベース: ネットワークトポロジー情報
/// - ソースルーターID: 計算の起点となるルーター
/// 
/// ## 出力
/// - ルーティングテーブル: 各宛先への最短パスとコスト
pub struct SPFCalculator;

impl SPFCalculator {
    /// LSAデータベースを基に、指定されたソースルーターからの最短パスを計算
    /// 
    /// # 引数
    /// * `lsa_database` - ネットワーク全体のLink State Advertisement情報
    /// * `source_router_id` - 計算の起点となるルーターのID
    /// 
    /// # 戻り値
    /// RoutingTableEntryのベクター。各エントリには宛先、コスト、ネクストホップが含まれる
    pub fn calculate_routes_from_lsa(
        lsa_database: &HashMap<String, LSA>,
        source_router_id: u32,
        topology: &NetworkTopology,  // Still needed for interface info
    ) -> (HashMap<u32, RoutingTableEntry>, std::collections::HashSet<u32>) {
        console_log!("SPF CALLED for router {}", source_router_id);
        let mut adjacencies: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();  // router_id -> [(neighbor_id, cost)]
        
        // Build adjacency graph from Router LSAs
        console_log!("SPF: Building adjacency graph from {} LSAs", lsa_database.len());
        console_log!("SPF: LSA database contents:");
        for (key, lsa) in lsa_database {
            console_log!("SPF:   Key: {} - Advertising Router: {}", key, lsa.header.advertising_router);
        }
        
        for lsa in lsa_database.values() {
            if let LSAData::Router(router_lsa) = &lsa.data {
                let advertising_router = lsa.header.advertising_router
                    .split('.')
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                
                // Debug Router ID extraction for all LSAs
                console_log!("SPF: Extracted router ID {} from advertising_router '{}'", 
                    advertising_router, lsa.header.advertising_router);
                console_log!("SPF: Router {} processing Router LSA from {} with {} links", 
                    source_router_id, advertising_router, router_lsa.links.len());
                
                console_log!("SPF: Processing Router LSA from router {} with {} links", 
                    advertising_router, router_lsa.links.len());
                    
                // Log all links in this LSA
                for (i, link) in router_lsa.links.iter().enumerate() {
                    console_log!("SPF:   Link {}: ID={}, Type={:?}, Metric={}", 
                        i, link.link_id, link.link_type, link.metric);
                }
                
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
            } else {
                console_log!("SPF: Skipping non-Router LSA from advertising router {}", 
                    lsa.header.advertising_router);
            }
        }
        
        // If no LSAs exist, return empty routing table
        if adjacencies.is_empty() {
            console_log!("SPF: No adjacencies found for router {}, returning empty routing table", source_router_id);
            console_log!("SPF: LSA database contained {} LSAs but no valid adjacencies built", lsa_database.len());
            console_log!("SPF EMPTY EXIT: Router {} no adjacencies", source_router_id);
            let mut reachable = std::collections::HashSet::new();
            reachable.insert(source_router_id);
            return (HashMap::new(), reachable);
        }
        
        console_log!("SPF: Found adjacencies for {} routers", adjacencies.len());
        
        // Debug: Show all adjacencies built for ALL routers
        console_log!("SPF Router {}: Adjacencies built:", source_router_id);
        for (router_id, neighbors) in &adjacencies {
            console_log!("  Router {} -> {:?}", router_id, neighbors);
        }
        
        // Check if source router has any adjacencies
        if !adjacencies.contains_key(&source_router_id) {
            console_log!("SPF WARNING: Source router {} has no adjacencies in LSA database!", source_router_id);
            console_log!("SPF: This may indicate missing or outdated LSA for source router");
        }
        
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
        
        // Debug: Show Dijkstra results for ALL routers
        console_log!("SPF Router {}: Dijkstra distances found:", source_router_id);
        for (dest_id, dist) in &distances {
            console_log!("  Router {} distance: {}", dest_id, dist);
        }
        console_log!("SPF Router {}: Next hops found:", source_router_id);
        for (dest_id, next_hop) in &next_hops {
            console_log!("  Router {} next hop: {}", dest_id, next_hop);
        }
        
        // Build routing table entries using topology for interface info
        for (dest_router_id, &next_hop) in &next_hops {
            if *dest_router_id == source_router_id {
                continue;
            }
            
            console_log!("SPF: Building route to router {} via next hop {}", dest_router_id, next_hop);
            
            // Debug: Route building attempt for ALL routers
            console_log!("SPF Router {}: Attempting to build route to {} via {}", 
                source_router_id, dest_router_id, next_hop);
            
            // Find interface from topology (excluding failed links)
            console_log!("SPF: Looking for interface from router {} to next hop {}", source_router_id, next_hop);
            
            let link_found = topology.links.values()
                .find(|link| {
                    let matches = !link.is_failed && (
                        (link.router1_id == source_router_id && link.router2_id == next_hop) ||
                        (link.router2_id == source_router_id && link.router1_id == next_hop)
                    );
                    if matches {
                        console_log!("SPF: Found matching link: router{}-router{} (interface {}-{})", 
                            link.router1_id, link.router2_id, 
                            link.router1_interface_id, link.router2_interface_id);
                    }
                    matches
                });
            
            let interface_info = link_found.and_then(|link| {
                let interface_id = if link.router1_id == source_router_id {
                    link.router1_interface_id
                } else {
                    link.router2_interface_id
                };
                
                console_log!("SPF: Looking for interface {} on router {}", interface_id, source_router_id);
                
                let interface = topology.routers.get(&source_router_id)
                    .and_then(|r| r.interfaces.get(&interface_id));
                
                if interface.is_none() {
                    console_log!("SPF: ERROR - Interface {} not found on router {}", interface_id, source_router_id);
                }
                
                interface
            });
            
            if let Some(interface) = interface_info {
                let entry = RoutingTableEntry {
                    destination: format!("{}.{}.{}.{}", 1, 1, 1, dest_router_id),
                    netmask: "255.255.255.255".to_string(),
                    next_hop: format!("{}.{}.{}.{}", 1, 1, 1, next_hop),
                    interface_id: interface.id,
                    interface_name: interface.name.clone(),
                    metric: distances[dest_router_id],
                    protocol: crate::router::RoutingProtocol::OSPF,
                };
                
                console_log!("SPF:   Route entry: {} via {} on interface {} (metric {})", 
                    entry.destination, entry.next_hop, entry.interface_id, entry.metric);
                console_log!("SPF Router {}: SUCCESS - Route to {} via {} added to routing table", 
                    source_router_id, dest_router_id, next_hop);
                
                routing_table.insert(*dest_router_id, entry);
            } else {
                console_log!("SPF Router {}: FAILED - No interface found for route to {} via {}", 
                    source_router_id, dest_router_id, next_hop);
                console_log!("SPF: Available links for router {}:", source_router_id);
                for link in topology.links.values() {
                    if link.router1_id == source_router_id || link.router2_id == source_router_id {
                        console_log!("SPF:   Link: router{}-router{} (interfaces {}-{}, failed: {})", 
                            link.router1_id, link.router2_id, 
                            link.router1_interface_id, link.router2_interface_id, link.is_failed);
                    }
                }
            }
        }
        
        // Collect all reachable routers (including those we found paths to)
        let mut reachable_routers = std::collections::HashSet::new();
        reachable_routers.insert(source_router_id); // Always include self
        
        // Add all routers we found distances to (even if we couldn't build routes)
        for router_id in distances.keys() {
            reachable_routers.insert(*router_id);
        }
        
        console_log!("SPF FINISHED for router {} with {} routes, {} reachable routers", 
            source_router_id, routing_table.len(), reachable_routers.len());
        console_log!("SPF Router {}: Reachable routers: {:?}", source_router_id, reachable_routers);
        for (dest_id, route) in &routing_table {
            console_log!("SPF Router {}:   Route to {} -> {} via interface {} (metric {})", 
                source_router_id, dest_id, route.next_hop, route.interface_id, route.metric);
        }
        
        (routing_table, reachable_routers)
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
                    interface_name: interface.name.clone(),
                    metric: distances[dest_router_id],
                    protocol: crate::router::RoutingProtocol::OSPF,
                };

                routing_table.insert(*dest_router_id, entry);
            }
        }

        routing_table
    }

}