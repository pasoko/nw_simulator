use wasm_bindgen_test::*;
use nw_simulator::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_simulator_creation_and_destruction() {
    let simulator = NetworkSimulator::new();
    let routers_json = simulator.get_routers_json();
    let routers: Vec<serde_json::Value> = serde_json::from_str(&routers_json).unwrap();
    assert_eq!(routers.len(), 0);
    // Simulator is automatically dropped here
}

#[wasm_bindgen_test]
fn test_router_lifecycle() {
    let mut simulator = NetworkSimulator::new();
    
    // Add routers
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    let _r2 = simulator.add_router("Router2".to_string(), 200.0, 200.0);
    
    let routers_json = simulator.get_routers_json();
    let routers: Vec<serde_json::Value> = serde_json::from_str(&routers_json).unwrap();
    assert_eq!(routers.len(), 2);
    
    // Get routers JSON
    let routers_json = simulator.get_routers_json();
    assert!(routers_json.contains("Router1"));
    assert!(routers_json.contains("Router2"));
    
    // Delete router
    assert!(simulator.delete_router(r1));
    let routers_json_after = simulator.get_routers_json();
    let routers_after: Vec<serde_json::Value> = serde_json::from_str(&routers_json_after).unwrap();
    assert_eq!(routers_after.len(), 1);
    
    // Try to delete non-existent router
    assert!(!simulator.delete_router(999));
}

#[wasm_bindgen_test]
fn test_connection_management() {
    let mut simulator = NetworkSimulator::new();
    
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    let r2 = simulator.add_router("Router2".to_string(), 200.0, 200.0);
    let r3 = simulator.add_router("Router3".to_string(), 300.0, 300.0);
    
    // Connect routers
    simulator.connect_routers(r1, r2, 10);
    simulator.connect_routers(r2, r3, 20);
    
    let connections_json = simulator.get_connections_json();
    let connections: serde_json::Value = serde_json::from_str(&connections_json).unwrap();
    assert_eq!(connections.as_array().unwrap().len(), 2);
    
    // Disconnect routers
    assert!(simulator.disconnect_routers(r1, r2));
    
    let connections_json = simulator.get_connections_json();
    let connections: serde_json::Value = serde_json::from_str(&connections_json).unwrap();
    assert_eq!(connections.as_array().unwrap().len(), 1);
}

#[wasm_bindgen_test]
fn test_ospf_enablement() {
    let mut simulator = NetworkSimulator::new();
    
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    let r2 = simulator.add_router("Router2".to_string(), 200.0, 200.0);
    
    simulator.connect_routers(r1, r2, 10);
    
    // Enable OSPF
    simulator.enable_ospf(r1);
    simulator.enable_ospf(r2);
    
    // Get router details to verify OSPF is enabled
    let details_json = simulator.get_router_details_json(r1);
    assert!(details_json.contains("\"ospf_enabled\":true"));
}

#[wasm_bindgen_test]
fn test_simulation_control() {
    let mut simulator = NetworkSimulator::new();
    
    // Create a simple network
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    let r2 = simulator.add_router("Router2".to_string(), 200.0, 200.0);
    simulator.connect_routers(r1, r2, 10);
    simulator.enable_ospf(r1);
    simulator.enable_ospf(r2);
    
    // Start simulation
    simulator.start_simulation();
    
    // Step simulation
    simulator.step_simulation(0.1);
    
    // Get simulation stats
    let stats_json = simulator.get_simulation_stats_json();
    assert!(stats_json.contains("\"running\":true"));
    
    // Stop simulation
    simulator.stop_simulation();
    
    let stats_json = simulator.get_simulation_stats_json();
    assert!(stats_json.contains("\"running\":false"));
}

#[wasm_bindgen_test]
fn test_failure_simulation() {
    let mut simulator = NetworkSimulator::new();
    
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    let r2 = simulator.add_router("Router2".to_string(), 200.0, 200.0);
    let r3 = simulator.add_router("Router3".to_string(), 300.0, 300.0);
    
    simulator.connect_routers(r1, r2, 10);
    simulator.connect_routers(r2, r3, 10);
    
    // Toggle link failure
    assert!(simulator.toggle_link_failure(r1, r2));
    
    let connections_json = simulator.get_connections_json();
    assert!(connections_json.contains("\"is_failed\":true"));
    
    // Toggle back
    assert!(simulator.toggle_link_failure(r1, r2));
    
    let connections_json = simulator.get_connections_json();
    assert!(connections_json.contains("\"is_failed\":false"));
    
    // Toggle router failure
    simulator.toggle_router_failure(r2);
    
    let router_json = simulator.get_router_details_json(r2);
    assert!(router_json.contains("\"is_failed\":true"));
}

#[wasm_bindgen_test]
fn test_event_retrieval() {
    let mut simulator = NetworkSimulator::new();
    
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    let r2 = simulator.add_router("Router2".to_string(), 200.0, 200.0);
    simulator.connect_routers(r1, r2, 10);
    simulator.enable_ospf(r1);
    simulator.enable_ospf(r2);
    
    simulator.start_simulation();
    simulator.step_simulation(1.0); // Step for 1 second
    
    // Get recent events
    let events_json = simulator.get_recent_events_json(10);
    let events: serde_json::Value = serde_json::from_str(&events_json).unwrap();
    
    // Should have some events (at least OSPF initialization)
    assert!(events.as_array().unwrap().len() > 0);
}

#[wasm_bindgen_test]
fn test_router_position_update() {
    let mut simulator = NetworkSimulator::new();
    
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    
    // Update position
    assert!(simulator.update_router_position(r1, 150.0, 150.0));
    
    let routers_json = simulator.get_routers_json();
    assert!(routers_json.contains("\"x\":150"));
    assert!(routers_json.contains("\"y\":150"));
    
    // Try to update non-existent router
    assert!(!simulator.update_router_position(999, 200.0, 200.0));
}

#[wasm_bindgen_test]
fn test_json_serialization_format() {
    let mut simulator = NetworkSimulator::new();
    
    let r1 = simulator.add_router("Router1".to_string(), 100.0, 100.0);
    simulator.enable_ospf(r1);
    
    // Test router summary JSON
    let summary_json = simulator.get_router_summary_json(r1);
    let summary: serde_json::Value = serde_json::from_str(&summary_json).unwrap();
    assert!(summary.get("id").is_some());
    assert!(summary.get("name").is_some());
    assert!(summary.get("ospf_enabled").is_some());
    
    // Test simulation stats JSON
    let stats_json = simulator.get_simulation_stats_json();
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    assert!(stats.get("running").is_some());
    assert!(stats.get("time").is_some());
    assert!(stats.get("event_count").is_some());
}