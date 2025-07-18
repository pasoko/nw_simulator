#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::stub_area::AreaType;
    
    #[test]
    fn test_virtual_link_configuration() {
        let mut sim = NetworkSimulation::new();
        
        // Create topology: R1 -- Area 0 -- R2 -- Area 1 -- R3 -- Area 2 -- R4
        //                                 ABR               ABR
        // Virtual link: R2 <---> R3 through Area 1
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0); // Backbone only
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0); // ABR (Area 0, 1)
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0); // ABR (Area 1, 2)
        let r4 = sim.add_router("R4".to_string(), 400.0, 100.0); // Area 2 only
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_routers(r3, r4, 10).unwrap();
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        sim.enable_ospf(r4).unwrap();
        
        // Configure areas (in a real implementation, interfaces would be in different areas)
        // For now, we test the virtual link mechanism
        
        // Configure virtual link between R2 and R3 through Area 1
        let result = sim.configure_virtual_link(r2, r3, "1.0.0.0".to_string());
        assert!(result.is_ok(), "Failed to configure virtual link: {:?}", result);
        
        let interface_id = result.unwrap();
        assert!(interface_id >= 1000, "Virtual link interface ID should be >= 1000");
        
        // Verify virtual link status
        let vlink_status = sim.get_virtual_link_status();
        assert!(!vlink_status.is_empty(), "Virtual link status should not be empty");
        
        // Find R2's virtual link status
        let r2_status = vlink_status.iter().find(|(id, _)| *id == r2);
        assert!(r2_status.is_some(), "R2 should have virtual link status");
        
        let (_, r2_vlinks) = r2_status.unwrap();
        assert_eq!(r2_vlinks.len(), 1, "R2 should have exactly one virtual link");
        
        // Verify virtual link details
        let (remote_id, transit_area, _state, is_up) = &r2_vlinks[0];
        assert!(remote_id.contains("1.1.1.3"), "Remote ID should be 1.1.1.3, got {}", remote_id);
        assert_eq!(transit_area, "1.0.0.0");
        assert!(!is_up, "Virtual link should be down initially");
    }
    
    #[test]
    fn test_virtual_link_removal() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Configure virtual link
        sim.configure_virtual_link(r1, r2, "1.0.0.0".to_string()).unwrap();
        
        // Remove virtual link
        let result = sim.remove_virtual_link(r1, r2);
        assert!(result.is_ok(), "Failed to remove virtual link");
        
        // Verify removal
        let vlink_status = sim.get_virtual_link_status();
        let r1_status = vlink_status.iter().find(|(id, _)| *id == r1);
        assert!(r1_status.is_none() || r1_status.unwrap().1.is_empty(), 
                "R1 should have no virtual links after removal");
    }
    
    #[test]
    fn test_virtual_link_stub_area_validation() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Configure R1's area as stub
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.area_id = "1.0.0.0".to_string();
            engine.configure_stub_area(AreaType::stub(10)).unwrap();
        }
        
        // Try to configure virtual link through stub area
        let result = sim.configure_virtual_link(r1, r2, "1.0.0.0".to_string());
        assert!(result.is_err(), "Virtual link through stub area should fail");
    }
    
    #[test]
    fn test_virtual_link_backbone_transit() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Try to configure virtual link through backbone (area 0)
        let result = sim.configure_virtual_link(r1, r2, "0.0.0.0".to_string());
        // This should fail because backbone cannot be transit area
        assert!(result.is_err(), "Virtual link configuration through backbone should fail");
    }
}