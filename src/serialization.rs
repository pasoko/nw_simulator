use serde_json;
use crate::simulation::NetworkSimulation;
use crate::ui_state::{UIState, RouterUI, ConnectionUI};
use crate::event_manager::SimulationEventType;

/// Serialization utilities for WebAssembly interface
/// 
/// Handles all JSON conversion logic to keep the WebAssembly interface clean
pub struct SerializationHelper;

impl SerializationHelper {
    /// Convert routers to JSON with UI positioning
    pub fn routers_to_json(simulation: &NetworkSimulation, ui_state: &UIState) -> String {
        let routers: Vec<RouterUI> = simulation.topology.routers.iter().map(|(id, state)| {
            let (x, y) = ui_state.get_router_position(id)
                .copied()
                .unwrap_or((0.0, 0.0));
            RouterUI {
                id: *id,
                name: state.name.clone(),
                x,
                y,
                ospf_enabled: state.ospf_state.is_some(),
                is_failed: state.is_failed,
            }
        }).collect();
        serde_json::to_string(&routers).unwrap_or_default()
    }

    /// Convert connections to JSON
    pub fn connections_to_json(simulation: &NetworkSimulation) -> String {
        let connections: Vec<ConnectionUI> = simulation.topology.links.values().map(|link| {
            ConnectionUI {
                from_router_id: link.router1_id,
                from_interface_id: link.router1_interface_id,
                to_router_id: link.router2_id,
                to_interface_id: link.router2_interface_id,
                cost: link.cost,
                is_failed: link.is_failed,
            }
        }).collect();
        serde_json::to_string(&connections).unwrap_or_default()
    }

    /// Convert recent events to JSON
    pub fn recent_events_to_json(simulation: &NetworkSimulation, count: usize) -> String {
        let events = simulation.get_recent_events(count);
        serde_json::to_string(&events).unwrap_or_default()
    }

    /// Create router summary JSON with OSPF details
    pub fn router_summary_to_json(simulation: &NetworkSimulation, router_id: u32) -> String {
        if let Some(router) = simulation.topology.routers.get(&router_id) {
            let neighbor_count = simulation.get_ospf_neighbor_count(router_id);
            let route_count = router.routing_table.len();
            
            // Get latest OSPF event for this router
            let latest_ospf_event = Self::find_latest_ospf_event(simulation, router_id);
            
            let summary = serde_json::json!({
                "id": router_id,
                "name": router.name,
                "ospf_enabled": router.ospf_state.is_some(),
                "neighbor_count": neighbor_count,
                "route_count": route_count,
                "latest_event": latest_ospf_event
            });
            
            serde_json::to_string(&summary).unwrap_or_default()
        } else {
            "{}".to_string()
        }
    }

    /// Create detailed router information JSON
    pub fn router_details_to_json(simulation: &NetworkSimulation, router_id: u32) -> String {
        if let Some(router) = simulation.topology.routers.get(&router_id) {
            let ospf_neighbor_count = simulation.get_ospf_neighbor_count(router_id);
            let lsa_count = simulation.get_ospf_lsa_count(router_id);
            
            // Convert interfaces HashMap to array for frontend compatibility
            let interfaces_array: Vec<_> = router.interfaces.values().collect();
            
            // Get OSPF neighbors details if OSPF is enabled
            let neighbors = if router.ospf_state.is_some() {
                Self::get_ospf_neighbors_details(simulation, router_id)
            } else {
                vec![]
            };
            
            // Get LSA database details if OSPF is enabled
            let lsa_database = if router.ospf_state.is_some() {
                Self::get_lsa_database_details(simulation, router_id)
            } else {
                vec![]
            };
            
            let details = serde_json::json!({
                "id": router.id,
                "name": router.name,
                "interfaces": interfaces_array,
                "routing_table": router.routing_table,
                "ospf_enabled": router.ospf_state.is_some(),
                "ospf_neighbors": ospf_neighbor_count,
                "lsa_database_size": lsa_count,
                "neighbors": neighbors,
                "lsa_database": lsa_database
            });
            serde_json::to_string(&details).unwrap_or_default()
        } else {
            "{}".to_string()
        }
    }

    /// Create simulation statistics JSON
    pub fn simulation_stats_to_json(simulation: &NetworkSimulation) -> String {
        let stats = serde_json::json!({
            "total_routers": simulation.topology.routers.len(),
            "total_links": simulation.topology.links.len(),
            "ospf_enabled_routers": simulation.topology.routers.values()
                .filter(|r| r.ospf_state.is_some()).count(),
            "simulation_time": simulation.simulation_time,
            "total_events": simulation.simulation_log().len(),
        });
        serde_json::to_string(&stats).unwrap_or_default()
    }

    /// Convert all events to JSON
    pub fn all_events_to_json(simulation: &NetworkSimulation) -> String {
        serde_json::to_string(simulation.simulation_log()).unwrap_or_default()
    }

    /// Helper function to find the latest OSPF event for a router
    fn find_latest_ospf_event(simulation: &NetworkSimulation, router_id: u32) -> String {
        let recent_events = simulation.get_recent_events(20);
        recent_events.iter()
            .filter(|e| Self::is_ospf_event_for_router(e, router_id))
            .last()
            .map(|e| e.description.clone())
            .unwrap_or_else(|| "No recent OSPF events".to_string())
    }

    /// Helper function to check if an event is OSPF-related for a specific router
    fn is_ospf_event_for_router(event: &crate::event_manager::SimulationEvent, router_id: u32) -> bool {
        match &event.event_type {
            SimulationEventType::OSPFEnabled { router_id: rid } => *rid == router_id,
            SimulationEventType::NeighborStateChanged { router_id: rid, .. } => *rid == router_id,
            SimulationEventType::RoutingTableUpdated { router_id: rid } => *rid == router_id,
            _ => false
        }
    }
    
    /// Get OSPF neighbors details for a router
    fn get_ospf_neighbors_details(simulation: &NetworkSimulation, router_id: u32) -> Vec<serde_json::Value> {
        // For now, return empty array since we need access to OSPF engine internals
        // This would need to be implemented with proper access to OSPF engine state
        vec![]
    }
    
    /// Get LSA database details for a router
    fn get_lsa_database_details(simulation: &NetworkSimulation, router_id: u32) -> Vec<serde_json::Value> {
        // For now, return empty array since we need access to OSPF engine internals
        // This would need to be implemented with proper access to OSPF engine state
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::NetworkSimulation;
    use crate::ui_state::UIState;

    #[test]
    fn test_serialization_helper() {
        let simulation = NetworkSimulation::new();
        let ui_state = UIState::new();
        
        // Test basic serialization
        let routers_json = SerializationHelper::routers_to_json(&simulation, &ui_state);
        assert_eq!(routers_json, "[]");
        
        let connections_json = SerializationHelper::connections_to_json(&simulation);
        assert_eq!(connections_json, "[]");
        
        let stats_json = SerializationHelper::simulation_stats_to_json(&simulation);
        assert!(stats_json.contains("total_routers"));
    }
}