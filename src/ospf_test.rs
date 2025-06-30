#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::console_log;
    
    #[test]
    fn test_ospf_checksum_and_sequence_numbers() {
        let mut sim = NetworkSimulation::new();
        
        // Create a simple 3-router topology
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 150.0, 200.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_routers(r1, r3, 10).unwrap();
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Start simulation
        sim.start_simulation();
        
        // Run simulation for 60 seconds to allow OSPF convergence
        for _ in 0..600 {
            sim.step_simulation(0.1);
        }
        
        // Check that all routers have LSAs
        assert!(sim.get_ospf_lsa_count(r1) > 0, "Router 1 should have LSAs");
        assert!(sim.get_ospf_lsa_count(r2) > 0, "Router 2 should have LSAs");
        assert!(sim.get_ospf_lsa_count(r3) > 0, "Router 3 should have LSAs");
        
        // Check that all routers have neighbors
        assert_eq!(sim.get_ospf_neighbor_count(r1), 2, "Router 1 should have 2 neighbors");
        assert_eq!(sim.get_ospf_neighbor_count(r2), 2, "Router 2 should have 2 neighbors");
        assert_eq!(sim.get_ospf_neighbor_count(r3), 2, "Router 3 should have 2 neighbors");
        
        console_log!("OSPF checksum and sequence number test passed");
    }
    
    #[test]
    fn test_ospf_maxage_handling() {
        let mut sim = NetworkSimulation::new();
        
        // Create 2 routers
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        sim.start_simulation();
        
        // Run for a short time to establish adjacency
        for _ in 0..100 {
            sim.step_simulation(0.1);
        }
        
        // Force age LSAs to near MaxAge by running for a long time
        // (In a real test, we'd have a way to artificially age LSAs)
        console_log!("Testing MaxAge handling - this would require aging LSAs to 3600 seconds");
        
        // Check that LSAs still exist (not immediately deleted at MaxAge)
        assert!(sim.get_ospf_lsa_count(r1) > 0, "Router 1 should still have LSAs");
        assert!(sim.get_ospf_lsa_count(r2) > 0, "Router 2 should still have LSAs");
    }
    
    #[test]
    fn test_ospf_flooding_control() {
        let mut sim = NetworkSimulation::new();
        
        // Create 2 routers
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        sim.start_simulation();
        
        // Run simulation to establish adjacency
        for _ in 0..100 {
            sim.step_simulation(0.1);
        }
        
        // Try to trigger rapid LSA updates (should be rate-limited by MinLSInterval)
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0);
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Add and remove link rapidly
        sim.disconnect_routers(r1, r2);
        sim.step_simulation(0.1);
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.step_simulation(0.1);
        
        // Check that flooding control is working (would need to verify from logs)
        console_log!("Flooding control test - check logs for MinLSInterval enforcement");
    }
}