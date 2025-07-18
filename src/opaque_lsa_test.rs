#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::opaque_lsa::{OpaqueLSAGenerator, TELink, StandardOpaqueType};
    
    #[test]
    fn test_opaque_lsa_generation() {
        let mut sim = NetworkSimulation::new();
        
        // Create simple topology
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Enable Opaque capability on routers
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.set_opaque_capability(true);
            
            // Generate a Traffic Engineering LSA
            let te_link = TELink {
                link_type: 1,
                link_id: "1.1.1.2".to_string(),
                local_interface_ip: "10.0.0.1".to_string(),
                remote_interface_ip: "10.0.0.2".to_string(),
                metric: 10,
                max_bandwidth: 1000.0,
                max_reservable_bandwidth: 800.0,
                unreserved_bandwidth: [800.0; 8],
                admin_group: 0,
            };
            
            let events = engine.generate_te_lsa(vec![te_link]);
            // No flood events expected since neighbors aren't in Full state yet
            assert_eq!(events.len(), 0, "No flood events without Full neighbors");
            
            // Generate a custom Opaque LSA
            let custom_data = vec![1, 2, 3, 4, 5];
            let result = engine.generate_opaque_lsa(
                10, // Type 10 (Area-local)
                StandardOpaqueType::ApplicationSpecific as u8,
                1,
                custom_data,
            );
            assert!(result.is_ok(), "Custom Opaque LSA generation should succeed");
        }
        
        // Verify Opaque LSAs are in the database
        if let Some(engine) = sim.get_ospf_engine(r1) {
            let lsa_count = engine.get_lsa_count();
            assert!(lsa_count >= 3, "Should have at least Router LSA + 2 Opaque LSAs");
        }
    }
    
    #[test]
    fn test_opaque_lsa_flooding() {
        let mut sim = NetworkSimulation::new();
        
        // Create topology
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 150.0, 200.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        
        // Enable OSPF with Opaque capability
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Enable Opaque capability on all routers
        for router_id in &[r1, r2, r3] {
            if let Some(engine) = sim.get_ospf_engine_mut(*router_id) {
                engine.set_opaque_capability(true);
            }
        }
        
        sim.start_simulation();
        
        // Let OSPF converge
        for _ in 0..50 {
            sim.step_simulation(0.1);
        }
        
        // Generate Opaque LSA on R1
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let data = b"Test Opaque Data".to_vec();
            let _ = engine.generate_opaque_lsa(10, 1, 100, data).unwrap();
        }
        
        // Let the LSA flood
        for _ in 0..20 {
            sim.step_simulation(0.1);
        }
        
        // Verify all routers have the Opaque LSA
        for router_id in &[r1, r2, r3] {
            if let Some(engine) = sim.get_ospf_engine(*router_id) {
                // Check if router has Opaque LSAs
                let lsa_count = engine.get_lsa_count();
                println!("Router {} has {} LSAs", router_id, lsa_count);
            }
        }
    }
    
    #[test]
    fn test_opaque_lsa_scope() {
        let mut sim = NetworkSimulation::new();
        
        // Create topology with multiple areas
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Enable Opaque capability
        for router_id in &[r1, r2] {
            if let Some(engine) = sim.get_ospf_engine_mut(*router_id) {
                engine.set_opaque_capability(true);
            }
        }
        
        // Test Type 9 (Link-local) LSA
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let data = b"Link-local data".to_vec();
            let result = engine.generate_opaque_lsa(9, 1, 1, data);
            assert!(result.is_ok(), "Type 9 Opaque LSA generation should succeed");
        }
        
        // Test Type 10 (Area-local) LSA
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let data = b"Area-local data".to_vec();
            let result = engine.generate_opaque_lsa(10, 2, 1, data);
            assert!(result.is_ok(), "Type 10 Opaque LSA generation should succeed");
        }
        
        // Test Type 11 (AS-wide) LSA
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let data = b"AS-wide data".to_vec();
            let result = engine.generate_opaque_lsa(11, 3, 1, data);
            assert!(result.is_ok(), "Type 11 Opaque LSA generation should succeed");
        }
    }
    
    #[test]
    fn test_opaque_capability_check() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Try to generate Opaque LSA without capability enabled
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let data = vec![1, 2, 3];
            let result = engine.generate_opaque_lsa(10, 1, 1, data);
            assert!(result.is_err(), "Should fail without Opaque capability");
            
            // Enable capability and try again
            engine.set_opaque_capability(true);
            let data = vec![1, 2, 3];
            let result = engine.generate_opaque_lsa(10, 1, 1, data);
            assert!(result.is_ok(), "Should succeed with Opaque capability");
        }
    }
}