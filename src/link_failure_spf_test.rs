#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;

    #[test]
    fn test_link_failure_triggers_spf() {
        // This test verifies that when a link fails, both affected routers
        // trigger SPF calculation (fixing the bug where only one router would update)
        let mut sim = NetworkSimulation::new();
        
        // Create routers
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0);
        
        // Connect routers in a line: R1 - R2 - R3
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Simulate link failure between R1 and R2
        // This is the core of the test - verifying the bug fix
        let result = sim.toggle_link_failure(r1, r2);
        assert!(result, "Link failure simulation should succeed");
        
        // The bug was that only one router would have SPF pending
        // After the fix, BOTH routers should have SPF pending
        
        // Check R1 has SPF pending
        if let Some(engine1) = sim.get_ospf_engine(r1) {
            assert!(engine1.is_spf_pending(), 
                "BUG FIX VERIFICATION: R1 should have SPF pending after its link failed");
        }
        
        // Check R2 has SPF pending  
        if let Some(engine2) = sim.get_ospf_engine(r2) {
            assert!(engine2.is_spf_pending(), 
                "BUG FIX VERIFICATION: R2 should have SPF pending after its link failed");
        }
    }

}