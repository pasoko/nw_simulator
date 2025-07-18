#[cfg(test)]
mod tests {
    use crate::ospf_tos::*;
    use crate::ospf_engine::OSPFEngine;
    use crate::simulation::NetworkSimulation;
    use crate::console_log;

    #[test]
    fn test_tos_value_creation() {
        // Test valid TOS values
        let tos_normal = TOSValue::new(0).unwrap();
        assert_eq!(tos_normal.value(), 0);
        assert!(tos_normal.is_normal());
        
        let tos_custom = TOSValue::new(16).unwrap();
        assert_eq!(tos_custom.value(), 16);
        assert!(!tos_custom.is_normal());
        
        // Test invalid TOS value
        let tos_invalid = TOSValue::new(128);
        assert!(tos_invalid.is_err());
        
        // Test predefined TOS values
        assert_eq!(TOSValue::normal().value(), 0);
        assert_eq!(TOSValue::minimize_cost().value(), 1);
        assert_eq!(TOSValue::maximize_reliability().value(), 2);
        assert_eq!(TOSValue::maximize_throughput().value(), 4);
        assert_eq!(TOSValue::minimize_delay().value(), 8);
    }

    #[test]
    fn test_tos_metric() {
        let metric = TOSMetric::new(TOSValue::minimize_delay(), 100);
        assert_eq!(metric.tos.value(), 8);
        assert_eq!(metric.metric, 100);
        
        let normal_metric = TOSMetric::normal(50);
        assert!(normal_metric.tos.is_normal());
        assert_eq!(normal_metric.metric, 50);
    }

    #[test]
    fn test_tos_capabilities() {
        let mut capabilities = TOSCapabilities::new();
        
        // Initially disabled
        assert!(!capabilities.tos_support_enabled);
        assert_eq!(capabilities.supported_tos_values.len(), 1);
        assert!(capabilities.is_tos_supported(&TOSValue::normal()));
        
        // Enable TOS support
        capabilities.enable_tos_support();
        assert!(capabilities.tos_support_enabled);
        
        // Add supported TOS values
        capabilities.add_supported_tos(TOSValue::minimize_delay());
        capabilities.add_supported_tos(TOSValue::maximize_throughput());
        assert_eq!(capabilities.supported_tos_values.len(), 3);
        assert!(capabilities.is_tos_supported(&TOSValue::minimize_delay()));
        assert!(capabilities.is_tos_supported(&TOSValue::maximize_throughput()));
        
        // Try to remove normal TOS (should fail)
        capabilities.remove_supported_tos(TOSValue::normal());
        assert!(capabilities.is_tos_supported(&TOSValue::normal()));
        
        // Remove other TOS
        capabilities.remove_supported_tos(TOSValue::minimize_delay());
        assert!(!capabilities.is_tos_supported(&TOSValue::minimize_delay()));
        
        // Disable TOS support
        capabilities.disable_tos_support();
        assert!(!capabilities.tos_support_enabled);
        assert_eq!(capabilities.supported_tos_values.len(), 1);
    }

    #[test]
    fn test_interface_tos_metrics() {
        let mut capabilities = TOSCapabilities::new();
        capabilities.enable_tos_support();
        capabilities.add_supported_tos(TOSValue::minimize_delay());
        capabilities.add_supported_tos(TOSValue::maximize_throughput());
        
        // Set interface metrics
        let metrics = vec![
            TOSMetric::new(TOSValue::normal(), 10),
            TOSMetric::new(TOSValue::minimize_delay(), 5),
            TOSMetric::new(TOSValue::maximize_throughput(), 20),
            TOSMetric::new(TOSValue::minimize_cost(), 30), // Not supported, should be filtered
        ];
        
        capabilities.set_interface_tos_metrics(1, metrics);
        
        // Get specific TOS metric
        assert_eq!(capabilities.get_interface_tos_metric(1, &TOSValue::normal()), Some(10));
        assert_eq!(capabilities.get_interface_tos_metric(1, &TOSValue::minimize_delay()), Some(5));
        assert_eq!(capabilities.get_interface_tos_metric(1, &TOSValue::maximize_throughput()), Some(20));
        assert_eq!(capabilities.get_interface_tos_metric(1, &TOSValue::minimize_cost()), None);
        
        // Get all metrics
        let all_metrics = capabilities.get_interface_all_tos_metrics(1);
        assert_eq!(all_metrics.len(), 3); // minimize_cost was filtered out
    }

    #[test]
    fn test_tos_routing_table() {
        let mut table = TOSRoutingTable::new();
        
        // Add routes
        let entry1 = TOSRoutingEntry {
            destination: "192.168.1.0".to_string(),
            mask: "255.255.255.0".to_string(),
            tos: TOSValue::normal(),
            cost: 10,
            next_hop: "10.0.0.2".to_string(),
            outgoing_interface: 1,
            advertising_router: "1.1.1.1".to_string(),
        };
        
        let entry2 = TOSRoutingEntry {
            destination: "192.168.1.0".to_string(),
            mask: "255.255.255.0".to_string(),
            tos: TOSValue::minimize_delay(),
            cost: 5,
            next_hop: "10.0.0.3".to_string(),
            outgoing_interface: 2,
            advertising_router: "1.1.1.1".to_string(),
        };
        
        table.add_route(entry1.clone());
        table.add_route(entry2.clone());
        
        // Get specific TOS route
        let route = table.get_route("192.168.1.0", TOSValue::normal()).unwrap();
        assert_eq!(route.cost, 10);
        assert_eq!(route.next_hop, "10.0.0.2");
        
        let route = table.get_route("192.168.1.0", TOSValue::minimize_delay()).unwrap();
        assert_eq!(route.cost, 5);
        assert_eq!(route.next_hop, "10.0.0.3");
        
        // Get all routes for destination
        let routes = table.get_all_tos_routes("192.168.1.0");
        assert_eq!(routes.len(), 2);
        
        // Remove route
        table.remove_route("192.168.1.0", TOSValue::normal());
        assert!(table.get_route("192.168.1.0", TOSValue::normal()).is_none());
        assert_eq!(table.route_count(), 1);
        
        // Clear table
        table.clear();
        assert_eq!(table.route_count(), 0);
    }

    #[test]
    fn test_router_link_with_tos() {
        let mut link = RouterLinkWithTOS::new(
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
            1,
            10,
        );
        
        assert_eq!(link.num_tos, 0);
        assert_eq!(link.metric, 10);
        
        // Add TOS metrics
        link.add_tos_metric(TOSValue::minimize_delay(), 5);
        link.add_tos_metric(TOSValue::maximize_throughput(), 20);
        assert_eq!(link.num_tos, 2);
        
        // Get TOS metrics
        assert_eq!(link.get_tos_metric(&TOSValue::normal()), 10);
        assert_eq!(link.get_tos_metric(&TOSValue::minimize_delay()), 5);
        assert_eq!(link.get_tos_metric(&TOSValue::maximize_throughput()), 20);
        assert_eq!(link.get_tos_metric(&TOSValue::minimize_cost()), 10); // Falls back to normal
        
        // Try to add normal TOS (should be ignored)
        link.add_tos_metric(TOSValue::normal(), 15);
        assert_eq!(link.num_tos, 2); // Still 2
    }

    #[test]
    fn test_summary_lsa_with_tos() {
        let mut lsa = SummaryLSAWithTOS::new("255.255.255.0".to_string(), 100);
        
        // Add TOS metrics
        lsa.add_tos_metric(TOSValue::minimize_delay(), 50);
        lsa.add_tos_metric(TOSValue::maximize_reliability(), 75);
        
        // Get metrics
        assert_eq!(lsa.get_tos_metric(&TOSValue::normal()), 100);
        assert_eq!(lsa.get_tos_metric(&TOSValue::minimize_delay()), 50);
        assert_eq!(lsa.get_tos_metric(&TOSValue::maximize_reliability()), 75);
        assert_eq!(lsa.get_tos_metric(&TOSValue::minimize_cost()), 100); // Falls back
    }

    #[test]
    fn test_as_external_lsa_with_tos() {
        let mut lsa = ASExternalLSAWithTOS::new(
            "255.255.255.0".to_string(),
            2, // E2
            1000,
            "0.0.0.0".to_string(),
            0,
        );
        
        // Add TOS metric
        lsa.add_tos_metric(
            TOSValue::minimize_cost(),
            1, // E1
            500,
            "10.0.0.1".to_string(),
            100,
        );
        
        assert_eq!(lsa.tos_metrics.len(), 1);
        assert_eq!(lsa.tos_metrics[0].tos.value(), 1);
        assert_eq!(lsa.tos_metrics[0].metric, 500);
        assert_eq!(lsa.tos_metrics[0].metric_type, 1);
    }

    #[test]
    fn test_ospf_engine_tos_integration() {
        let mut engine = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        // Initially TOS disabled
        assert!(!engine.is_tos_enabled());
        assert!(!engine.get_area_options().get_t_bit());
        
        // Enable TOS support
        engine.enable_tos_support();
        assert!(engine.is_tos_enabled());
        assert!(engine.get_area_options().get_t_bit());
        
        // Add supported TOS values
        engine.add_supported_tos(TOSValue::minimize_delay());
        engine.add_supported_tos(TOSValue::maximize_throughput());
        
        let supported = engine.get_supported_tos_values();
        assert_eq!(supported.len(), 3); // normal + 2 added
        assert!(engine.is_tos_supported(&TOSValue::minimize_delay()));
        
        // Set interface TOS metrics
        let metrics = vec![
            TOSMetric::new(TOSValue::normal(), 10),
            TOSMetric::new(TOSValue::minimize_delay(), 5),
        ];
        engine.set_interface_tos_metrics(1, metrics);
        
        // Get metrics
        assert_eq!(engine.get_interface_tos_metric(1, &TOSValue::normal()), Some(10));
        assert_eq!(engine.get_interface_tos_metric(1, &TOSValue::minimize_delay()), Some(5));
        
        // Get all metrics
        let all_metrics = engine.get_interface_all_tos_metrics(1);
        assert_eq!(all_metrics.len(), 2);
        
        // Remove TOS support
        engine.remove_supported_tos(TOSValue::minimize_delay());
        assert!(!engine.is_tos_supported(&TOSValue::minimize_delay()));
        
        // Disable TOS support
        engine.disable_tos_support();
        assert!(!engine.is_tos_enabled());
        assert!(!engine.get_area_options().get_t_bit());
        assert_eq!(engine.get_tos_routing_table().route_count(), 0);
    }

    #[test]
    fn test_tos_in_simulation() {
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Enable TOS on R1
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.enable_tos_support();
            engine.add_supported_tos(TOSValue::minimize_delay());
            
            // Set interface metrics
            let interface_id = 1; // Assume interface 1
            let metrics = vec![
                TOSMetric::new(TOSValue::normal(), 10),
                TOSMetric::new(TOSValue::minimize_delay(), 5),
            ];
            engine.set_interface_tos_metrics(interface_id, metrics);
            
            console_log!("R1 TOS enabled: {}", engine.is_tos_enabled());
            console_log!("R1 T-bit: {}", engine.get_area_options().get_t_bit());
        }
        
        // Check that R2 can see R1's TOS capability through options
        if let Some(engine1) = sim.get_ospf_engine(r1) {
            if let Some(engine2) = sim.get_ospf_engine(r2) {
                let options1 = engine1.get_area_options();
                let options2 = engine2.get_area_options();
                
                assert!(options1.get_t_bit());
                assert!(!options2.get_t_bit()); // R2 hasn't enabled TOS
                
                console_log!("R1 options: {}", options1.to_string());
                console_log!("R2 options: {}", options2.to_string());
            }
        }
    }

    #[test]
    fn test_tos_backward_compatibility() {
        let mut capabilities = TOSCapabilities::new();
        
        // Even with TOS disabled, normal TOS should work
        assert!(capabilities.is_tos_supported(&TOSValue::normal()));
        
        // Set metrics with only normal TOS
        capabilities.set_interface_tos_metrics(1, vec![TOSMetric::normal(10)]);
        assert_eq!(capabilities.get_interface_tos_metric(1, &TOSValue::normal()), Some(10));
        
        // Non-normal TOS queries return None when TOS is disabled
        assert_eq!(capabilities.get_interface_tos_metric(1, &TOSValue::minimize_delay()), None);
    }
}