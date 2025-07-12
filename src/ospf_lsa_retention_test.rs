#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::console_log;

    #[test]
    fn test_lsa_retention_after_link_failure() {
        console_log!("=== Testing LSA retention after link failures ===");
        
        // Create a network with 5 routers in a specific topology
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 200.0, 200.0);
        let r4 = sim.add_router("R4".to_string(), 100.0, 200.0);
        let r5 = sim.add_router("R5".to_string(), 150.0, 150.0);
        
        // Create topology: R1-R2-R3-R4 ring with R5 connected to R1 and R4
        sim.connect_routers(r1, r2, 1);
        sim.connect_routers(r2, r3, 2);
        sim.connect_routers(r3, r4, 3);
        sim.connect_routers(r4, r1, 4);
        sim.connect_routers(r4, r5, 1);
        sim.connect_routers(r5, r1, 1);
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1);
        sim.enable_ospf(r2);
        sim.enable_ospf(r3);
        sim.enable_ospf(r4);
        sim.enable_ospf(r5);
        
        // Run simulation until convergence (need enough time for Hello, DD exchange, and SPF)
        for _ in 0..200 {
            sim.step_simulation(0.5);
        }
        
        // Check initial state - R1 should know about R4
        let r1_router = sim.topology.routers.get(&r1).unwrap();
        let r1_routes_initial = &r1_router.routing_table;
        let has_route_to_r4 = r1_routes_initial.iter().any(|route| 
            route.destination.contains(&format!("1.1.1.{}", r4))
        );
        assert!(has_route_to_r4, "R1 should have route to R4 initially");
        
        // Record initial LSA count for R1
        let initial_lsa_count = sim.get_ospf_lsa_count(r1);
        console_log!("R1 initial LSA count: {}", initial_lsa_count);
        assert!(initial_lsa_count >= 5, "R1 should have LSAs from all routers");
        
        // Fail link R4-R5
        sim.toggle_link_failure(r4, r5);
        console_log!("Failed link R4-R5");
        
        // Run simulation
        for _ in 0..50 {
            sim.step_simulation(0.5);
        }
        
        // Fail link R4-R1
        sim.toggle_link_failure(r4, r1);
        console_log!("Failed link R4-R1");
        
        // Run simulation
        for _ in 0..50 {
            sim.step_simulation(0.5);
        }
        
        // Check that R1 still has R4's LSA in database
        let lsa_count_after_failures = sim.get_ospf_lsa_count(r1);
        console_log!("R1 LSA count after failures: {}", lsa_count_after_failures);
        
        // R1 should still have LSAs from all routers (including R4)
        // because R4 is still reachable via R1-R2-R3-R4
        assert!(lsa_count_after_failures >= 5, 
            "R1 should retain LSAs from all routers including R4");
        
        // Check that R1 still has route to R4
        let r1_router_after = sim.topology.routers.get(&r1).unwrap();
        let r1_routes_after = &r1_router_after.routing_table;
        let still_has_route_to_r4 = r1_routes_after.iter().any(|route| 
            route.destination.contains(&format!("1.1.1.{}", r4))
        );
        assert!(still_has_route_to_r4, 
            "R1 should still have route to R4 via R2-R3 path");
        
        console_log!("Test passed: LSAs are properly retained after link failures");
    }
}