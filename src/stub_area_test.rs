#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::stub_area::{AreaType};
    use crate::router::{LSA, LSAHeader, LSAType, LSAData, ASExternalLSA};
    
    #[test]
    fn test_stub_area_configuration() {
        let mut sim = NetworkSimulation::new();
        
        // Create routers in different areas
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0); // Area 0 (backbone)
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0); // ABR
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0); // Area 1 (stub)
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Configure area 1 as stub area
        // Note: In a real implementation, r3 would be in area 1, not area 0
        // For now, we'll test the stub area functionality without the backbone restriction
        if let Some(engine) = sim.get_ospf_engine_mut(r3) {
            // First change the area ID to non-backbone
            engine.area_id = "1.0.0.0".to_string();
            
            let result = engine.configure_stub_area(AreaType::stub(10));
            assert!(result.is_ok(), "Failed to configure stub area");
            
            // Verify area type
            assert!(matches!(engine.get_area_type(), AreaType::Stub { .. }));
        }
        
        // ABR should also be configured for area 1 as stub
        if let Some(engine) = sim.get_ospf_engine_mut(r2) {
            // Note: In a real implementation, ABR would have multiple OSPF engines,
            // one per area. For simplicity, we're testing the concept here.
            engine.area_id = "1.0.0.0".to_string();
            let result = engine.configure_stub_area(AreaType::stub(10));
            assert!(result.is_ok());
        }
    }
    
    #[test]
    fn test_stub_area_lsa_filtering() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure as stub area
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.area_id = "1.0.0.0".to_string();
            engine.configure_stub_area(AreaType::stub(10)).unwrap();
            
            // Create a Type 5 (AS-External) LSA
            let external_lsa = LSA {
                header: LSAHeader {
                    ls_age: 0,
                    ls_type: LSAType::ASExternalLSA,
                    link_state_id: "10.0.0.0".to_string(),
                    advertising_router: "2.2.2.2".to_string(),
                    ls_sequence_number: 1,
                    ls_checksum: 0,
                    length: 36,
                },
                data: LSAData::ASExternal(ASExternalLSA {
                    network_mask: "255.255.255.0".to_string(),
                    metric: 10,
                    metric_type: 1,
                    forwarding_address: "0.0.0.0".to_string(),
                    external_route_tag: 0,
                    tos: 0,
                    tos_metric: 0,
                }),
            };
            
            // Type 5 LSA should be rejected in stub area
            assert!(!engine.should_accept_lsa_for_area(&external_lsa));
            
            // Create a Type 1 (Router) LSA
            let router_lsa = LSA {
                header: LSAHeader {
                    ls_age: 0,
                    ls_type: LSAType::RouterLSA,
                    link_state_id: "1.1.1.1".to_string(),
                    advertising_router: "1.1.1.1".to_string(),
                    ls_sequence_number: 1,
                    ls_checksum: 0,
                    length: 24,
                },
                data: LSAData::Router(crate::router::RouterLSA {
                    flags: 0,
                    num_links: 0,
                    links: vec![],
                }),
            };
            
            // Type 1 LSA should be accepted in stub area
            assert!(engine.should_accept_lsa_for_area(&router_lsa));
        }
    }
    
    #[test]
    fn test_totally_stub_area() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure as totally stub area
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.area_id = "1.0.0.0".to_string();
            engine.configure_stub_area(AreaType::totally_stub(20)).unwrap();
            
            // Create a Type 3 (Summary) LSA
            let summary_lsa = LSA {
                header: LSAHeader {
                    ls_age: 0,
                    ls_type: LSAType::SummaryLSA,
                    link_state_id: "192.168.1.0".to_string(),
                    advertising_router: "2.2.2.2".to_string(),
                    ls_sequence_number: 1,
                    ls_checksum: 0,
                    length: 28,
                },
                data: LSAData::Summary(crate::router::SummaryLSA {
                    network_mask: "255.255.255.0".to_string(),
                    metric: 10,
                    tos: 0,
                    tos_metric: 0,
                }),
            };
            
            // Type 3 LSA should be rejected in totally stub area (non-ABR)
            assert!(!engine.should_accept_lsa_for_area(&summary_lsa));
        }
    }
    
    #[test]
    fn test_backbone_cannot_be_stub() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Try to configure backbone (area 0) as stub
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            // This should fail because backbone cannot be stub
            let result = engine.configure_stub_area(AreaType::stub(10));
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Backbone area (0.0.0.0) cannot be configured as stub");
        }
    }
    
    #[test]
    fn test_abr_default_route_generation() {
        let mut sim = NetworkSimulation::new();
        
        // Create ABR scenario
        let r1 = sim.add_router("ABR".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            // Simulate ABR by adding multiple areas
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string()); // Backbone
            areas.insert("1.0.0.0".to_string()); // Area 1
            
            engine.update_abr_status(areas);
            
            // Now configure area 1 as stub (in real implementation, this would be
            // on the Area 1 instance of the OSPF engine)
            engine.area_id = "1.0.0.0".to_string();
            engine.configure_stub_area(AreaType::stub(10)).unwrap();
            
            // Check that default route LSA was generated
            let lsa_count = engine.get_lsa_count();
            assert!(lsa_count >= 2); // At least Router LSA + default route
            
            // In a full implementation, we would check for the specific
            // Type 3 LSA with link_state_id = "0.0.0.0"
        }
    }
    
    #[test]
    fn test_nssa_area_configuration() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure as NSSA
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.area_id = "1.0.0.0".to_string();
            let result = engine.configure_stub_area(AreaType::nssa(true, 15));
            assert!(result.is_ok(), "Failed to configure NSSA");
            
            // Verify area type
            assert!(matches!(engine.get_area_type(), AreaType::NSSA { .. }));
            
            // NSSA should reject Type 5 LSAs
            let external_lsa = LSA {
                header: LSAHeader {
                    ls_age: 0,
                    ls_type: LSAType::ASExternalLSA,
                    link_state_id: "10.0.0.0".to_string(),
                    advertising_router: "2.2.2.2".to_string(),
                    ls_sequence_number: 1,
                    ls_checksum: 0,
                    length: 36,
                },
                data: LSAData::ASExternal(ASExternalLSA {
                    network_mask: "255.255.255.0".to_string(),
                    metric: 10,
                    metric_type: 1,
                    forwarding_address: "0.0.0.0".to_string(),
                    external_route_tag: 0,
                    tos: 0,
                    tos_metric: 0,
                }),
            };
            
            assert!(!engine.should_accept_lsa_for_area(&external_lsa));
            
            // NSSA should accept Type 7 LSAs (Opaque AS-Wide can represent Type 7)
            // In a full implementation, we would have a specific Type 7 LSA
        }
    }
}