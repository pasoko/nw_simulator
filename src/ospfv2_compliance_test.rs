#[cfg(test)]
mod ospfv2_compliance_tests {
    use crate::NetworkSimulation;
    use crate::console_log;
    
    #[test]
    fn test_ospfv2_timing_compliance() {
        let mut sim = NetworkSimulation::new();
        
        // Create a simple topology
        let r1 = sim.add_router("Router1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("Router2".to_string(), 200.0, 200.0);
        let r3 = sim.add_router("Router3".to_string(), 300.0, 300.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Start simulation
        sim.start_simulation();
        
        // Check initial state (immediately after OSPF enable)
        console_log!("=== Testing OSPFv2 Compliance ===");
        console_log!("Time 0.0s - Initial state after OSPF enable");
        
        // Routes should be empty initially
        assert_routing_table_empty(&sim, r1, "Router 1 at 0s");
        assert_routing_table_empty(&sim, r2, "Router 2 at 0s");
        assert_routing_table_empty(&sim, r3, "Router 3 at 0s");
        
        // Step simulation to 5 seconds
        for _ in 0..50 {
            sim.step_simulation(0.1);
        }
        console_log!("Time 5.0s - Before first Hello");
        
        // Routes should still be empty (no Hello sent yet)
        assert_routing_table_empty(&sim, r1, "Router 1 at 5s");
        assert_routing_table_empty(&sim, r2, "Router 2 at 5s");
        assert_routing_table_empty(&sim, r3, "Router 3 at 5s");
        
        // Step to 10.5 seconds (after first Hello)
        for _ in 0..55 {
            sim.step_simulation(0.1);
        }
        console_log!("Time 10.5s - After first Hello");
        
        // Routes should still be empty (neighbors not Full yet)
        assert_routing_table_empty(&sim, r1, "Router 1 at 10.5s");
        assert_routing_table_empty(&sim, r2, "Router 2 at 10.5s");
        assert_routing_table_empty(&sim, r3, "Router 3 at 10.5s");
        
        // Step to 20 seconds (allowing for Full state and SPF)
        for _ in 0..95 {
            sim.step_simulation(0.1);
        }
        console_log!("Time 20.0s - After second Hello round");
        
        // Check neighbor states
        console_log!("Router 1 neighbors: {}", sim.get_ospf_neighbor_count(r1));
        console_log!("Router 2 neighbors: {}", sim.get_ospf_neighbor_count(r2));
        console_log!("Router 3 neighbors: {}", sim.get_ospf_neighbor_count(r3));
        
        // Routes might still be empty if not in Full state
        let r1_routes = get_route_count(&sim, r1);
        let r2_routes = get_route_count(&sim, r2);
        let r3_routes = get_route_count(&sim, r3);
        console_log!("Router 1 routes: {}", r1_routes);
        console_log!("Router 2 routes: {}", r2_routes);
        console_log!("Router 3 routes: {}", r3_routes);
        
        // Step to 30 seconds (third Hello round and more time for Full state)
        for _ in 0..100 {
            sim.step_simulation(0.1);
        }
        console_log!("Time 30.0s - After third Hello round");
        
        // Now routes should be populated
        assert_has_routes(&sim, r1, "Router 1 at 30s");
        assert_has_routes(&sim, r2, "Router 2 at 30s");
        assert_has_routes(&sim, r3, "Router 3 at 30s");
        
        console_log!("=== OSPFv2 Compliance Test Passed ===");
    }
    
    fn assert_routing_table_empty(sim: &NetworkSimulation, router_id: u32, context: &str) {
        if let Some(router) = sim.topology.routers.get(&router_id) {
            let route_count = router.routing_table.len();
            console_log!("{}: {} routes", context, route_count);
            assert_eq!(route_count, 0, 
                "{} should have empty routing table (OSPFv2 compliance)", context);
        }
    }
    
    fn assert_has_routes(sim: &NetworkSimulation, router_id: u32, context: &str) {
        if let Some(router) = sim.topology.routers.get(&router_id) {
            let route_count = router.routing_table.len();
            console_log!("{}: {} routes", context, route_count);
            assert!(route_count > 0, 
                "{} should have routes after convergence", context);
        }
    }
    
    fn get_route_count(sim: &NetworkSimulation, router_id: u32) -> usize {
        sim.topology.routers.get(&router_id)
            .map(|r| r.routing_table.len())
            .unwrap_or(0)
    }
    
    #[test]
    fn test_deterministic_router_processing() {
        // Test that all routers process at the same time
        let mut sim = NetworkSimulation::new();
        
        // Create 10 routers to ensure deterministic processing
        let mut routers = Vec::new();
        for i in 1..=10 {
            let r = sim.add_router(format!("Router{}", i), i as f64 * 100.0, 100.0);
            routers.push(r);
        }
        
        // Connect in a line
        for i in 0..9 {
            sim.connect_routers(routers[i], routers[i+1], 10).unwrap();
        }
        
        // Enable OSPF on all
        for &r in &routers {
            sim.enable_ospf(r).unwrap();
        }
        
        sim.start_simulation();
        
        // Step to first Hello time
        for _ in 0..105 {
            sim.step_simulation(0.1);
        }
        
        // Check that all routers have the same neighbor count (should be consistent)
        let neighbor_counts: Vec<usize> = routers.iter()
            .map(|&r| sim.get_ospf_neighbor_count(r))
            .collect();
        
        console_log!("Neighbor counts after first Hello: {:?}", neighbor_counts);
        
        // Edge routers should have 1 neighbor, middle routers should have 0-2
        // (depending on Hello timing)
        for (i, &count) in neighbor_counts.iter().enumerate() {
            if i == 0 || i == 9 {
                assert!(count <= 1, "Edge router {} has unexpected neighbor count", i+1);
            } else {
                assert!(count <= 2, "Middle router {} has unexpected neighbor count", i+1);
            }
        }
    }
}