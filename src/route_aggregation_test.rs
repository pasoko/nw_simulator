#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    
    #[test]
    fn test_inter_area_route_aggregation() {
        let mut sim = NetworkSimulation::new();
        
        // Create ABR scenario: R1 (Area 0) -- R2 (ABR) -- R3 (Area 1)
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0); // Area 0
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0); // ABR 
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0); // Area 1
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Configure R2 as ABR by adding multiple areas
        if let Some(engine) = sim.get_ospf_engine_mut(r2) {
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string()); // Backbone
            areas.insert("1.0.0.0".to_string()); // Area 1
            engine.update_abr_status(areas);
        }
        
        // Configure inter-area route aggregation on R2
        let result = sim.configure_route_aggregation(
            r2,
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()), // Area 1
            true, // Suppress more specific routes
            None, // Calculate metric automatically
        );
        assert!(result.is_ok(), "Failed to configure inter-area aggregation: {:?}", result);
        
        // Verify aggregation is configured
        let configs = sim.get_aggregation_config();
        assert!(!configs.is_empty(), "No aggregation configured");
        
        let r2_config = configs.iter().find(|(id, _)| *id == r2);
        assert!(r2_config.is_some(), "R2 should have aggregation configured");
        
        let (_, aggregates) = r2_config.unwrap();
        assert_eq!(aggregates.len(), 1, "R2 should have exactly one aggregate");
        
        let (network, mask, suppress, area_id, active) = &aggregates[0];
        assert_eq!(network, "192.168.0.0");
        assert_eq!(mask, "255.255.0.0");
        assert!(suppress, "Suppress should be enabled");
        assert_eq!(area_id, &Some("1.0.0.0".to_string()));
        assert!(!active, "Aggregate should be inactive without contributing routes");
    }
    
    #[test]
    fn test_external_route_aggregation() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure R1 as ASBR by adding external routes
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            // Simulate ASBR by adding external routes
            engine.add_external_route(
                "10.1.0.0".to_string(),
                "255.255.0.0".to_string(),
                10,
                crate::as_external_lsa::ExternalMetricType::Type1,
                "0.0.0.0".to_string(),
                0,
            );
            
            // Update ABR/ASBR status
            engine.update_abr_status(std::collections::HashSet::new());
        }
        
        // Configure external route aggregation
        let result = sim.configure_route_aggregation(
            r1,
            "10.0.0.0".to_string(),
            "255.0.0.0".to_string(),
            None, // External route (no area)
            true, // Suppress more specific routes
            Some(50), // Fixed metric
        );
        assert!(result.is_ok(), "Failed to configure external aggregation: {:?}", result);
        
        // Verify configuration
        let configs = sim.get_aggregation_config();
        let r1_config = configs.iter().find(|(id, _)| *id == r1);
        assert!(r1_config.is_some(), "R1 should have aggregation configured");
        
        let (_, aggregates) = r1_config.unwrap();
        let (network, mask, suppress, area_id, _active) = &aggregates[0];
        assert_eq!(network, "10.0.0.0");
        assert_eq!(mask, "255.0.0.0");
        assert!(suppress);
        assert_eq!(area_id, &None); // External route
    }
    
    #[test]
    fn test_aggregation_with_contributing_routes() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure ABR status
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string());
            areas.insert("1.0.0.0".to_string());
            engine.update_abr_status(areas);
        }
        
        // Configure aggregation
        sim.configure_route_aggregation(
            r1,
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            false, // Don't suppress (for testing)
            None,
        ).unwrap();
        
        // Simulate adding routes that match the aggregate
        if let Some(router) = sim.topology.routers.get_mut(&r1) {
            router.routing_table.push(crate::router::RoutingTableEntry {
                destination: "192.168.1.0".to_string(),
                netmask: "255.255.255.0".to_string(),
                next_hop: "10.0.0.2".to_string(),
                interface_id: 1,
                interface_name: "eth0".to_string(),
                metric: 10,
                protocol: crate::router::RoutingProtocol::OSPF,
            });
            
            router.routing_table.push(crate::router::RoutingTableEntry {
                destination: "192.168.2.0".to_string(),
                netmask: "255.255.255.0".to_string(),
                next_hop: "10.0.0.2".to_string(),
                interface_id: 1,
                interface_name: "eth0".to_string(),
                metric: 20,
                protocol: crate::router::RoutingProtocol::OSPF,
            });
        }
        
        // Update aggregation calculations
        sim.update_aggregation_calculations();
        
        // Check aggregation statistics
        let stats = sim.get_aggregation_statistics();
        assert!(!stats.is_empty(), "Should have aggregation statistics");
        
        let r1_stats = stats.iter().find(|(id, _)| *id == r1);
        assert!(r1_stats.is_some(), "R1 should have aggregation statistics");
        
        let (_, agg_stats) = r1_stats.unwrap();
        assert_eq!(agg_stats.total_aggregates, 1);
        assert_eq!(agg_stats.active_aggregates, 1);
        assert_eq!(agg_stats.inter_area_aggregates, 1);
        assert_eq!(agg_stats.external_aggregates, 0);
    }
    
    #[test]
    fn test_route_suppression() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure ABR
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string());
            areas.insert("1.0.0.0".to_string());
            engine.update_abr_status(areas);
        }
        
        // Configure aggregation with suppression
        sim.configure_route_aggregation(
            r1,
            "172.16.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true, // Enable suppression
            None,
        ).unwrap();
        
        // Add contributing routes
        if let Some(router) = sim.topology.routers.get_mut(&r1) {
            router.routing_table.push(crate::router::RoutingTableEntry {
                destination: "172.16.1.0".to_string(),
                netmask: "255.255.255.0".to_string(),
                next_hop: "10.0.0.2".to_string(),
                interface_id: 1,
                interface_name: "eth0".to_string(),
                metric: 15,
                protocol: crate::router::RoutingProtocol::OSPF,
            });
        }
        
        // Update calculations
        sim.update_aggregation_calculations();
        
        // Test route suppression
        if let Some(engine) = sim.get_ospf_engine(r1) {
            // Should suppress more specific routes
            assert!(engine.should_suppress_route("172.16.1.0", "255.255.255.0"));
            assert!(engine.should_suppress_route("172.16.100.0", "255.255.255.0"));
            
            // Should not suppress unrelated routes
            assert!(!engine.should_suppress_route("10.1.1.0", "255.255.255.0"));
            assert!(!engine.should_suppress_route("192.168.1.0", "255.255.255.0"));
        }
    }
    
    #[test]
    fn test_aggregation_removal() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure ABR
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string());
            areas.insert("1.0.0.0".to_string());
            engine.update_abr_status(areas);
        }
        
        // Configure aggregation
        sim.configure_route_aggregation(
            r1,
            "10.0.0.0".to_string(),
            "255.0.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true,
            None,
        ).unwrap();
        
        // Verify it's configured
        let configs = sim.get_aggregation_config();
        assert!(!configs.is_empty());
        
        // Remove aggregation
        let result = sim.remove_route_aggregation(
            r1,
            "10.0.0.0".to_string(),
            "255.0.0.0".to_string(),
        );
        assert!(result.is_ok(), "Failed to remove aggregation");
        
        // Verify it's removed
        let configs = sim.get_aggregation_config();
        let r1_config = configs.iter().find(|(id, _)| *id == r1);
        assert!(r1_config.is_none() || r1_config.unwrap().1.is_empty(),
                "Aggregation should be removed");
    }
    
    #[test]
    fn test_abr_requirement_validation() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Try to configure inter-area aggregation without ABR status
        let result = sim.configure_route_aggregation(
            r1,
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true,
            None,
        );
        assert!(result.is_err(), "Should fail without ABR status");
        
        // Configure as ABR and try again
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string());
            areas.insert("1.0.0.0".to_string());
            engine.update_abr_status(areas);
        }
        
        let result = sim.configure_route_aggregation(
            r1,
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true,
            None,
        );
        assert!(result.is_ok(), "Should succeed with ABR status");
    }
    
    #[test]
    fn test_metric_calculation() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        sim.enable_ospf(r1).unwrap();
        
        // Configure ABR
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            let mut areas = std::collections::HashSet::new();
            areas.insert("0.0.0.0".to_string());
            areas.insert("1.0.0.0".to_string());
            engine.update_abr_status(areas);
        }
        
        // Test fixed metric
        sim.configure_route_aggregation(
            r1,
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            false,
            Some(100), // Fixed metric
        ).unwrap();
        
        // Test automatic metric calculation
        sim.configure_route_aggregation(
            r1,
            "172.16.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            false,
            None, // Automatic metric
        ).unwrap();
        
        // Add routes with different metrics
        if let Some(router) = sim.topology.routers.get_mut(&r1) {
            router.routing_table.push(crate::router::RoutingTableEntry {
                destination: "172.16.1.0".to_string(),
                netmask: "255.255.255.0".to_string(),
                next_hop: "10.0.0.2".to_string(),
                interface_id: 1,
                interface_name: "eth0".to_string(),
                metric: 25, // Higher metric
                protocol: crate::router::RoutingProtocol::OSPF,
            });
            
            router.routing_table.push(crate::router::RoutingTableEntry {
                destination: "172.16.2.0".to_string(),
                netmask: "255.255.255.0".to_string(),
                next_hop: "10.0.0.2".to_string(),
                interface_id: 1,
                interface_name: "eth0".to_string(),
                metric: 15, // Lower metric (should be used for aggregate)
                protocol: crate::router::RoutingProtocol::OSPF,
            });
        }
        
        // Update calculations
        sim.update_aggregation_calculations();
        
        // Verify configurations
        let configs = sim.get_aggregation_config();
        let r1_config = configs.iter().find(|(id, _)| *id == r1).unwrap();
        assert_eq!(r1_config.1.len(), 2, "Should have two aggregates configured");
    }
}