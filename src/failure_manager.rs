use std::collections::BTreeMap;
use crate::network::NetworkTopology;
use crate::event_manager::EventManager;
use crate::ospf_engine::OSPFEngine;
use crate::console_log;

/// Failure Simulation Management
/// 
/// Handles router and link failure simulation including:
/// - Router failure/recovery
/// - Link failure/recovery
/// - OSPF neighbor state updates
/// - Event scheduling for failure scenarios
pub struct FailureManager {
    current_time: f64,
}

impl FailureManager {
    pub fn new() -> Self {
        FailureManager {
            current_time: 0.0,
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }
    
    /// Toggle link failure state and handle OSPF updates
    pub fn toggle_link_failure(
        &mut self,
        from_id: u32,
        to_id: u32,
        topology: &mut NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) -> (bool, Vec<crate::protocol::PacketEvent>) {
        console_log!("=== FAILURE MANAGER: toggle_link_failure called for link {}-{} ===", from_id, to_id);
        
        // Find the link
        let link_id = topology.links
            .iter()
            .find(|(_, link)| {
                (link.router1_id == from_id && link.router2_id == to_id) ||
                (link.router1_id == to_id && link.router2_id == from_id)
            })
            .map(|(id, _)| *id);
        
        console_log!("  Found link_id: {:?}", link_id);
        
        if let Some(link_id) = link_id {
            let (link_failed, _link_cost) = if let Some(link) = topology.links.get_mut(&link_id) {
                link.is_failed = !link.is_failed;
                console_log!("  Link failure state toggled: is_failed = {}", link.is_failed);
                (link.is_failed, link.cost)
            } else {
                console_log!("  ERROR: Link not found in topology");
                return (false, Vec::new());
            };
            
            // Log the event and handle failure/recovery
            let events = if link_failed {
                console_log!("  Processing link FAILURE");
                event_manager.log_link_failure(from_id, to_id);
                self.handle_link_failure(from_id, to_id, topology, ospf_engines, event_manager)
            } else {
                console_log!("  Processing link RECOVERY");
                event_manager.log_link_recovery(from_id, to_id);
                self.handle_link_recovery(from_id, to_id, topology, ospf_engines, event_manager)
            };
            
            console_log!("  Generated {} events", events.len());
            console_log!("=== FAILURE MANAGER: toggle_link_failure completed ===");
            
            (true, events)
        } else {
            console_log!("  ERROR: Link between {} and {} not found", from_id, to_id);
            (false, Vec::new())
        }
    }
    
    /// Toggle router failure state and handle OSPF updates
    pub fn toggle_router_failure(
        &mut self,
        router_id: u32,
        topology: &mut NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) -> bool {
        let (router_failed, router_name, has_ospf) = if let Some(router) = topology.routers.get_mut(&router_id) {
            router.is_failed = !router.is_failed;
            (router.is_failed, router.name.clone(), router.ospf_state.is_some())
        } else {
            return false;
        };
        
        // Log the event
        if router_failed {
            event_manager.log_router_failure(router_id, router_name.clone());
            self.handle_router_failure(router_id, topology, ospf_engines, event_manager);
        } else {
            event_manager.log_router_recovery(router_id, router_name.clone());
            self.handle_router_recovery(router_id, has_ospf, topology, ospf_engines, event_manager);
        }
        
        true
    }
    
    fn handle_link_failure(
        &mut self,
        from_id: u32,
        to_id: u32,
        topology: &NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) -> Vec<crate::protocol::PacketEvent> {
        console_log!("Processing link failure between {} and {}", from_id, to_id);
        
        // Get link information before removing neighbors
        let link_info = topology.links.values()
            .find(|link| {
                (link.router1_id == from_id && link.router2_id == to_id) ||
                (link.router1_id == to_id && link.router2_id == from_id)
            })
            .map(|link| {
                (
                    (link.router1_id, link.router2_id, link.router1_interface_id, link.cost),
                    (link.router2_id, link.router1_id, link.router2_interface_id, link.cost)
                )
            });
        
        if let Some(((r1_id, r2_id, _if1_id, _cost1), (_r2_id_rev, _r1_id_rev, _if2_id, _cost2))) = link_info {
            let mut events = Vec::new();
            
            // Process Router 1 link failure
            console_log!("Processing Router {} link failure to Router {}", r1_id, r2_id);
            if let Some(engine1) = ospf_engines.get_mut(&r1_id) {
                // Get current neighbor state before removing
                let prev_state = engine1.get_neighbor_state(r2_id)
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "Unknown".to_string());
                
                console_log!("Router {} neighbor {} previous state: {}", r1_id, r2_id, prev_state);
                
                let removed = engine1.remove_neighbor(r2_id);
                console_log!("Router {} remove_neighbor({}) returned: {}", r1_id, r2_id, removed);
                
                if removed {
                    event_manager.log_neighbor_state_changed(
                        r1_id, r2_id, prev_state, "Down".to_string()
                    );
                }
                
                // Always remove link and regenerate LSA, regardless of neighbor removal result
                console_log!("Router {} calling remove_link({})", r1_id, r2_id);
                let link_events = engine1.remove_link(r2_id);
                console_log!("Router {} remove_link({}) generated {} events", r1_id, r2_id, link_events.len());
                events.extend(link_events);
                
                // Log LSA regeneration event
                let lsa_count = engine1.get_lsa_count();
                event_manager.log_lsa_regenerated(r1_id, lsa_count);
                
                // Force SPF calculation
                console_log!("Router {} requesting SPF calculation after link failure", r1_id);
                engine1.request_spf_calculation();
            } else {
                console_log!("Warning: Router {} engine not found", r1_id);
            }
            
            // Process Router 2 link failure
            console_log!("Processing Router {} link failure to Router {}", r2_id, r1_id);
            if let Some(engine2) = ospf_engines.get_mut(&r2_id) {
                // Get current neighbor state before removing
                let prev_state = engine2.get_neighbor_state(r1_id)
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "Unknown".to_string());
                
                console_log!("Router {} neighbor {} previous state: {}", r2_id, r1_id, prev_state);
                
                let removed = engine2.remove_neighbor(r1_id);
                console_log!("Router {} remove_neighbor({}) returned: {}", r2_id, r1_id, removed);
                
                if removed {
                    event_manager.log_neighbor_state_changed(
                        r2_id, r1_id, prev_state, "Down".to_string()
                    );
                }
                
                // Always remove link and regenerate LSA, regardless of neighbor removal result
                console_log!("Router {} calling remove_link({})", r2_id, r1_id);
                let link_events = engine2.remove_link(r1_id);
                console_log!("Router {} remove_link({}) generated {} events", r2_id, r1_id, link_events.len());
                events.extend(link_events);
                
                // Log LSA regeneration event
                let lsa_count = engine2.get_lsa_count();
                event_manager.log_lsa_regenerated(r2_id, lsa_count);
                
                // Force SPF calculation
                console_log!("Router {} requesting SPF calculation after link failure", r2_id);
                engine2.request_spf_calculation();
            } else {
                console_log!("Warning: Router {} engine not found", r2_id);
            }
            
            // Log final event summary
            console_log!("Link failure processing complete, {} events generated", events.len());
            for (i, event) in events.iter().enumerate() {
                console_log!("  Event {}: {} -> {}", i + 1, event.from_router_id, event.to_router_id);
            }
            
            return events;
        } else {
            console_log!("Warning: Link between {} and {} not found", from_id, to_id);
        }
        Vec::new()
    }
    
    fn handle_link_recovery(
        &mut self,
        from_id: u32,
        to_id: u32,
        topology: &NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        _event_manager: &mut EventManager,
    ) -> Vec<crate::protocol::PacketEvent> {
        console_log!("Processing link recovery between {} and {}", from_id, to_id);
        
        // Get link information
        let link_info = topology.links.values()
            .find(|link| {
                (link.router1_id == from_id && link.router2_id == to_id) ||
                (link.router1_id == to_id && link.router2_id == from_id)
            })
            .map(|link| {
                (
                    (link.router1_id, link.router2_id, link.router1_interface_id, link.cost),
                    (link.router2_id, link.router1_id, link.router2_interface_id, link.cost)
                )
            });
        
        if let Some(((r1_id, r2_id, if1_id, cost1), (_r2_id_rev, _r1_id_rev, if2_id, cost2))) = link_info {
            // Add links back to OSPF engines
            if let Some(engine1) = ospf_engines.get_mut(&r1_id) {
                engine1.add_link(r2_id, if1_id, cost1);
            }
            
            if let Some(engine2) = ospf_engines.get_mut(&r2_id) {
                engine2.add_link(r1_id, if2_id, cost2);
            }
            
            console_log!("Link recovery complete - neighbor relationships will be re-established through Hello protocol");
        }
        Vec::new()  // LSA regeneration will happen after neighbors are re-established
    }
    
    fn handle_router_failure(
        &mut self,
        router_id: u32,
        topology: &mut NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) {
        console_log!("Processing router failure for router {}", router_id);
        
        // Clear routing table
        if let Some(router) = topology.routers.get_mut(&router_id) {
            router.routing_table.clear();
        }
        
        // Remove OSPF engine (it will be recreated on recovery)
        ospf_engines.remove(&router_id);
        
        // Notify all neighbors about this router going down
        let neighbors: Vec<u32> = topology.links
            .values()
            .filter_map(|link| {
                if link.router1_id == router_id {
                    Some(link.router2_id)
                } else if link.router2_id == router_id {
                    Some(link.router1_id)
                } else {
                    None
                }
            })
            .collect();
        
        let neighbor_count = neighbors.len();
        for neighbor_id in neighbors {
            if let Some(engine) = ospf_engines.get_mut(&neighbor_id) {
                // Get current neighbor state before removing
                let prev_state = engine.get_neighbor_state(router_id)
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "Unknown".to_string());
                
                if engine.remove_neighbor(router_id) {
                    event_manager.log_neighbor_state_changed(
                        neighbor_id, router_id, prev_state, "Down".to_string()
                    );
                }
            }
        }
        
        console_log!("Router {} failure processed - {} neighbors notified", router_id, neighbor_count);
    }
    
    fn handle_router_recovery(
        &mut self,
        router_id: u32,
        had_ospf: bool,
        topology: &mut NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        _event_manager: &mut EventManager,
    ) {
        console_log!("Processing router recovery for router {}", router_id);
        
        // Router recovery - recreate OSPF engine if enabled
        if had_ospf {
            let router_ip = format!("{}.{}.{}.{}", 1, 1, 1, router_id);
            let mut ospf_engine = OSPFEngine::new(router_ip.clone(), "0.0.0.0".to_string());
            
            // Add router links to OSPF engine
            for link in topology.links.values() {
                if !link.is_failed {
                    if link.router1_id == router_id {
                        ospf_engine.add_router_link(link.router2_id, link.router1_interface_id, link.cost);
                    } else if link.router2_id == router_id {
                        ospf_engine.add_router_link(link.router1_id, link.router2_interface_id, link.cost);
                    }
                }
            }
            
            ospf_engines.insert(router_id, ospf_engine);
            console_log!("Router {} OSPF engine recreated", router_id);
        }
    }
    
    fn schedule_lsa_regeneration(&mut self, router_ids: Vec<u32>, ospf_engines: &mut BTreeMap<u32, OSPFEngine>) {
        for router_id in router_ids {
            if let Some(engine) = ospf_engines.get_mut(&router_id) {
                // Only regenerate LSA if we have neighbors and router links
                if engine.get_neighbor_count() > 0 {
                    let events = engine.regenerate_router_lsa();
                    console_log!("Router {} regenerated LSA, {} flooding events generated", 
                        router_id, events.len());
                    // Note: Events need to be scheduled by the simulation engine
                }
            }
        }
    }
    
    /// Simulate a cascading failure scenario
    pub fn simulate_cascading_failure(
        &mut self,
        initial_router: u32,
        _delay_seconds: f64,
        topology: &mut NetworkTopology,
        ospf_engines: &mut BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) -> Vec<u32> {
        let mut failed_routers = Vec::new();
        
        // Start with initial router
        if self.toggle_router_failure(initial_router, topology, ospf_engines, event_manager) {
            failed_routers.push(initial_router);
            console_log!("Cascading failure started with router {}", initial_router);
            
            // Find neighbors to fail next (simplified - in practice would need scheduling)
            let neighbors: Vec<u32> = topology.links
                .values()
                .filter_map(|link| {
                    if link.router1_id == initial_router {
                        Some(link.router2_id)
                    } else if link.router2_id == initial_router {
                        Some(link.router1_id)
                    } else {
                        None
                    }
                })
                .collect();
            
            console_log!("Potential cascade targets: {:?}", neighbors);
        }
        
        failed_routers
    }
    
    /// Check for isolated routers after failures
    pub fn check_network_partitions(&self, topology: &NetworkTopology) -> Vec<Vec<u32>> {
        let mut partitions = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        for router_id in topology.routers.keys() {
            if !visited.contains(router_id) && !topology.routers[router_id].is_failed {
                let partition = self.find_connected_component(*router_id, topology, &mut visited);
                if !partition.is_empty() {
                    partitions.push(partition);
                }
            }
        }
        
        if partitions.len() > 1 {
            console_log!("Network partitioned into {} components", partitions.len());
            for (i, partition) in partitions.iter().enumerate() {
                console_log!("  Partition {}: {:?}", i + 1, partition);
            }
        }
        
        partitions
    }
    
    fn find_connected_component(
        &self,
        start_router: u32,
        topology: &NetworkTopology,
        visited: &mut std::collections::HashSet<u32>,
    ) -> Vec<u32> {
        let mut component = Vec::new();
        let mut stack = vec![start_router];
        
        while let Some(router_id) = stack.pop() {
            if visited.contains(&router_id) || topology.routers.get(&router_id).map_or(true, |r| r.is_failed) {
                continue;
            }
            
            visited.insert(router_id);
            component.push(router_id);
            
            // Find connected neighbors through non-failed links
            for link in topology.links.values() {
                if link.is_failed {
                    continue;
                }
                
                let neighbor = if link.router1_id == router_id {
                    Some(link.router2_id)
                } else if link.router2_id == router_id {
                    Some(link.router1_id)
                } else {
                    None
                };
                
                if let Some(neighbor_id) = neighbor {
                    if !visited.contains(&neighbor_id) {
                        stack.push(neighbor_id);
                    }
                }
            }
        }
        
        component
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::NetworkTopology;

    #[test]
    fn test_failure_manager_creation() {
        let manager = FailureManager::new();
        assert_eq!(manager.current_time, 0.0);
    }
    
    #[test] 
    fn test_network_partition_detection() {
        let manager = FailureManager::new();
        let mut topology = NetworkTopology::new();
        
        // Create simple network
        let r1 = topology.add_router("R1".to_string());
        let r2 = topology.add_router("R2".to_string());
        let _r3 = topology.add_router("R3".to_string());
        
        // Connect R1-R2
        topology.connect_routers(r1, r2, 10).unwrap();
        
        // R3 is isolated
        let partitions = manager.check_network_partitions(&topology);
        assert_eq!(partitions.len(), 2); // Two partitions: [R1,R2] and [R3]
    }
}