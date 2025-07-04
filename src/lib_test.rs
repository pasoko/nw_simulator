#[cfg(test)]
mod tests {
    use crate::NetworkSimulator;
    use wasm_bindgen_test::*;
    
    // Configure wasm_bindgen_test to run tests in the browser
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_network_simulator_creation() {
        let simulator = NetworkSimulator::new();
        assert!(simulator.get_routers_json() == "[]");
        assert!(simulator.get_connections_json() == "[]");
    }

    #[wasm_bindgen_test]
    fn test_add_router() {
        let mut simulator = NetworkSimulator::new();
        
        // Add a router
        let router_id = simulator.add_router("Router1".to_string(), 100.0, 200.0);
        assert!(router_id > 0);
        
        // Verify router was added
        let routers_json = simulator.get_routers_json();
        assert!(routers_json.contains("Router1"));
        assert!(routers_json.contains("100"));
        assert!(routers_json.contains("200"));
    }

    #[wasm_bindgen_test]
    fn test_connect_routers() {
        let mut simulator = NetworkSimulator::new();
        
        // Add two routers
        let router1_id = simulator.add_router("Router1".to_string(), 100.0, 100.0);
        let router2_id = simulator.add_router("Router2".to_string(), 200.0, 200.0);
        
        // Connect them
        simulator.connect_routers(router1_id, router2_id, 10);
        
        // Verify connection was created
        let connections_json = simulator.get_connections_json();
        assert!(connections_json.contains(&router1_id.to_string()));
        assert!(connections_json.contains(&router2_id.to_string()));
    }

    #[wasm_bindgen_test]
    fn test_delete_router() {
        let mut simulator = NetworkSimulator::new();
        
        // Add and then delete a router
        let router_id = simulator.add_router("RouterToDelete".to_string(), 50.0, 50.0);
        assert!(simulator.delete_router(router_id));
        
        // Verify router was deleted
        let routers_json = simulator.get_routers_json();
        assert!(!routers_json.contains("RouterToDelete"));
    }

    #[wasm_bindgen_test]
    fn test_update_router_position() {
        let mut simulator = NetworkSimulator::new();
        
        // Add a router
        let router_id = simulator.add_router("Router1".to_string(), 100.0, 100.0);
        
        // Update its position
        assert!(simulator.update_router_position(router_id, 300.0, 400.0));
        
        // Verify position was updated
        let routers_json = simulator.get_routers_json();
        assert!(routers_json.contains("300"));
        assert!(routers_json.contains("400"));
    }

    #[wasm_bindgen_test]
    fn test_disconnect_routers() {
        let mut simulator = NetworkSimulator::new();
        
        // Add two routers and connect them
        let router1_id = simulator.add_router("Router1".to_string(), 100.0, 100.0);
        let router2_id = simulator.add_router("Router2".to_string(), 200.0, 200.0);
        simulator.connect_routers(router1_id, router2_id, 10);
        
        // Disconnect them
        assert!(simulator.disconnect_routers(router1_id, router2_id));
        
        // Verify disconnection
        let connections_json = simulator.get_connections_json();
        assert!(connections_json == "[]");
    }

    #[wasm_bindgen_test]
    fn test_enable_ospf() {
        let mut simulator = NetworkSimulator::new();
        
        // Add a router and enable OSPF
        let router_id = simulator.add_router("OSPFRouter".to_string(), 150.0, 150.0);
        simulator.enable_ospf(router_id);
        
        // Verify OSPF is enabled by checking router details
        let router_details = simulator.get_router_details_json(router_id);
        assert!(router_details.contains("ospf_enabled"));
    }

    #[wasm_bindgen_test]
    fn test_simulation_control() {
        let mut simulator = NetworkSimulator::new();
        
        // Add some routers
        let router1_id = simulator.add_router("Router1".to_string(), 100.0, 100.0);
        let router2_id = simulator.add_router("Router2".to_string(), 200.0, 200.0);
        simulator.connect_routers(router1_id, router2_id, 10);
        
        // Start and stop simulation
        simulator.start_simulation();
        simulator.step_simulation(0.1);
        simulator.stop_simulation();
        
        // Test should not panic
    }

    #[wasm_bindgen_test]
    fn test_toggle_failures() {
        let mut simulator = NetworkSimulator::new();
        
        // Add routers and connect them
        let router1_id = simulator.add_router("Router1".to_string(), 100.0, 100.0);
        let router2_id = simulator.add_router("Router2".to_string(), 200.0, 200.0);
        simulator.connect_routers(router1_id, router2_id, 10);
        
        // Toggle link failure
        assert!(simulator.toggle_link_failure(router1_id, router2_id));
        
        // Toggle router failure
        assert!(simulator.toggle_router_failure(router1_id));
    }

    #[wasm_bindgen_test]
    fn test_get_json_methods() {
        let mut simulator = NetworkSimulator::new();
        
        // Add some data
        let router_id = simulator.add_router("TestRouter".to_string(), 100.0, 100.0);
        
        // Test all JSON getter methods
        assert!(!simulator.get_routers_json().is_empty());
        assert!(!simulator.get_recent_events_json(10).is_empty());
        assert!(!simulator.get_router_summary_json(router_id).is_empty());
        assert!(!simulator.get_all_events_json().is_empty());
        assert!(!simulator.get_router_details_json(router_id).is_empty());
        assert!(!simulator.get_simulation_stats_json().is_empty());
    }

    #[wasm_bindgen_test]
    fn test_invalid_operations() {
        let mut simulator = NetworkSimulator::new();
        
        // Test operations on non-existent routers
        assert!(!simulator.delete_router(999));
        assert!(!simulator.update_router_position(999, 0.0, 0.0));
        assert!(!simulator.disconnect_routers(998, 999));
        assert!(!simulator.toggle_router_failure(999));
        assert!(!simulator.toggle_link_failure(998, 999));
    }

    #[wasm_bindgen_test]
    fn test_router_position_bounds() {
        let mut simulator = NetworkSimulator::new();
        
        // Test extreme position values
        let router1 = simulator.add_router("EdgeRouter1".to_string(), 0.0, 0.0);
        let router2 = simulator.add_router("EdgeRouter2".to_string(), f64::MAX, f64::MAX);
        let router3 = simulator.add_router("EdgeRouter3".to_string(), -1000.0, -1000.0);
        
        assert!(router1 > 0);
        assert!(router2 > 0);
        assert!(router3 > 0);
        
        // Update with extreme values
        assert!(simulator.update_router_position(router1, f64::MIN, f64::MIN));
    }

    #[wasm_bindgen_test]
    fn test_multiple_connections() {
        let mut simulator = NetworkSimulator::new();
        
        // Create a small network topology
        let r1 = simulator.add_router("Core1".to_string(), 200.0, 200.0);
        let r2 = simulator.add_router("Edge1".to_string(), 100.0, 100.0);
        let r3 = simulator.add_router("Edge2".to_string(), 300.0, 100.0);
        let r4 = simulator.add_router("Edge3".to_string(), 200.0, 300.0);
        
        // Create star topology
        simulator.connect_routers(r1, r2, 10);
        simulator.connect_routers(r1, r3, 20);
        simulator.connect_routers(r1, r4, 15);
        
        // Verify all connections
        let connections = simulator.get_connections_json();
        assert!(connections.contains(&r1.to_string()));
        assert!(connections.contains(&r2.to_string()));
        assert!(connections.contains(&r3.to_string()));
        assert!(connections.contains(&r4.to_string()));
    }

    #[wasm_bindgen_test]
    fn test_simulation_time_progression() {
        let mut simulator = NetworkSimulator::new();
        
        // Setup simple topology
        let r1 = simulator.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = simulator.add_router("R2".to_string(), 200.0, 200.0);
        simulator.connect_routers(r1, r2, 10);
        simulator.enable_ospf(r1);
        simulator.enable_ospf(r2);
        
        // Start simulation
        simulator.start_simulation();
        
        // Step through time
        for _ in 0..10 {
            simulator.step_simulation(0.1);
        }
        
        // Stop simulation
        simulator.stop_simulation();
        
        // Verify we can get events
        let events = simulator.get_recent_events_json(10);
        assert!(!events.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_ospf_configuration() {
        let mut simulator = NetworkSimulator::new();
        
        // Create routers
        let r1 = simulator.add_router("OSPFRouter1".to_string(), 100.0, 100.0);
        let r2 = simulator.add_router("OSPFRouter2".to_string(), 200.0, 200.0);
        
        // Connect and enable OSPF
        simulator.connect_routers(r1, r2, 10);
        simulator.enable_ospf(r1);
        simulator.enable_ospf(r2);
        
        // Get router details
        let r1_details = simulator.get_router_details_json(r1);
        let r2_details = simulator.get_router_details_json(r2);
        
        // Verify OSPF is enabled
        assert!(r1_details.contains("ospf_enabled"));
        assert!(r2_details.contains("ospf_enabled"));
    }

    #[wasm_bindgen_test]
    fn test_large_network_creation() {
        let mut simulator = NetworkSimulator::new();
        
        // Create 20 routers
        let mut routers = Vec::new();
        for i in 0..20 {
            let x = (i % 5) as f64 * 100.0;
            let y = (i / 5) as f64 * 100.0;
            let id = simulator.add_router(format!("Router{}", i), x, y);
            routers.push(id);
        }
        
        // Create mesh connections (each router connects to 3 others)
        for i in 0..routers.len() {
            for j in 1..=3 {
                let target = (i + j) % routers.len();
                if i != target {
                    simulator.connect_routers(routers[i], routers[target], 10 + j as u32);
                }
            }
        }
        
        // Verify network was created
        let routers_json = simulator.get_routers_json();
        let connections_json = simulator.get_connections_json();
        
        // Should have 20 routers
        for i in 0..20 {
            assert!(routers_json.contains(&format!("Router{}", i)));
        }
        
        // Should have multiple connections
        assert!(connections_json.len() > 100); // Rough check for non-empty connections
    }

    #[wasm_bindgen_test]
    fn test_failure_recovery_scenario() {
        let mut simulator = NetworkSimulator::new();
        
        // Create triangle topology
        let r1 = simulator.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = simulator.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = simulator.add_router("R3".to_string(), 150.0, 200.0);
        
        simulator.connect_routers(r1, r2, 10);
        simulator.connect_routers(r2, r3, 10);
        simulator.connect_routers(r3, r1, 10);
        
        // Enable OSPF
        simulator.enable_ospf(r1);
        simulator.enable_ospf(r2);
        simulator.enable_ospf(r3);
        
        // Simulate link failure
        assert!(simulator.toggle_link_failure(r1, r2));
        
        // Verify failure state can be toggled back
        assert!(simulator.toggle_link_failure(r1, r2));
        
        // Simulate router failure
        assert!(simulator.toggle_router_failure(r2));
        
        // Verify router failure can be recovered
        assert!(simulator.toggle_router_failure(r2));
    }

    #[wasm_bindgen_test]
    fn test_json_serialization_formats() {
        let mut simulator = NetworkSimulator::new();
        
        // Create simple network
        let r1 = simulator.add_router("TestRouter".to_string(), 100.0, 100.0);
        
        // Test all JSON getter methods return valid JSON
        let jsons = vec![
            simulator.get_routers_json(),
            simulator.get_connections_json(),
            simulator.get_recent_events_json(5),
            simulator.get_router_summary_json(r1),
            simulator.get_all_events_json(),
            simulator.get_router_details_json(r1),
            simulator.get_simulation_stats_json(),
        ];
        
        // Basic JSON validation - should start with [ or {
        for json in jsons {
            assert!(json.starts_with('[') || json.starts_with('{'), 
                    "Invalid JSON format: {}", json);
        }
    }
}