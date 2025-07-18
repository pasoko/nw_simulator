#[cfg(test)]
mod tests {
    use crate::ospf_options::OSPFOptions;
    use crate::ospf_engine::OSPFEngine;
    use crate::simulation::NetworkSimulation;
    use crate::console_log;

    #[test]
    fn test_ospf_options_bit_operations() {
        let mut options = OSPFOptions::new();
        
        // Test initial state
        assert_eq!(options.as_byte(), 0);
        assert!(!options.get_e_bit());
        assert!(!options.get_mc_bit());
        assert!(!options.get_np_bit());
        assert!(!options.get_dc_bit());
        assert!(!options.get_o_bit());
        
        // Test setting individual bits
        options.set_e_bit(true);
        assert!(options.get_e_bit());
        assert_eq!(options.as_byte(), 0x02);
        
        options.set_mc_bit(true);
        assert!(options.get_mc_bit());
        assert_eq!(options.as_byte(), 0x06); // 0x02 | 0x04
        
        options.set_o_bit(true);
        assert!(options.get_o_bit());
        assert_eq!(options.as_byte(), 0x46); // 0x02 | 0x04 | 0x40
        
        // Test clearing bits
        options.set_mc_bit(false);
        assert!(!options.get_mc_bit());
        assert_eq!(options.as_byte(), 0x42); // 0x02 | 0x40
    }

    #[test]
    fn test_area_type_options() {
        // Test standard area options
        let standard = OSPFOptions::standard_area_options();
        assert!(standard.get_e_bit());
        assert!(standard.get_o_bit());
        assert!(!standard.get_np_bit());
        
        // Test stub area options
        let stub = OSPFOptions::stub_area_options();
        assert!(!stub.get_e_bit());
        assert!(stub.get_o_bit());
        assert!(!stub.get_np_bit());
        
        // Test NSSA area options
        let nssa = OSPFOptions::nssa_area_options();
        assert!(!nssa.get_e_bit());
        assert!(nssa.get_np_bit());
        assert!(nssa.get_o_bit());
    }

    #[test]
    fn test_options_compatibility() {
        let standard1 = OSPFOptions::standard_area_options();
        let standard2 = OSPFOptions::standard_area_options();
        let stub = OSPFOptions::stub_area_options();
        let nssa = OSPFOptions::nssa_area_options();
        
        // Same area types should be compatible
        assert!(standard1.is_compatible_with(&standard2));
        
        // Different area types should not be compatible
        assert!(!standard1.is_compatible_with(&stub));
        assert!(!standard1.is_compatible_with(&nssa));
        assert!(!stub.is_compatible_with(&nssa));
        
        // Test with additional optional bits
        let mut standard_with_mc = OSPFOptions::standard_area_options();
        standard_with_mc.set_mc_bit(true);
        
        // Should still be compatible (MC bit is optional)
        assert!(standard1.is_compatible_with(&standard_with_mc));
    }

    #[test]
    fn test_options_to_string() {
        let mut options = OSPFOptions::new();
        assert_eq!(options.to_string(), "None");
        
        options.set_e_bit(true);
        assert_eq!(options.to_string(), "E");
        
        options.set_mc_bit(true);
        assert_eq!(options.to_string(), "MC,E");
        
        options.set_o_bit(true);
        assert_eq!(options.to_string(), "O,MC,E");
        
        options.set_np_bit(true);
        assert_eq!(options.to_string(), "O,N/P,MC,E");
    }

    #[test]
    fn test_ospf_engine_area_options() {
        let mut engine = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        // Test default options
        let default_options = engine.get_area_options();
        assert!(default_options.get_e_bit());
        assert!(!default_options.get_o_bit()); // O-bit is now disabled by default
        
        // Test stub area configuration
        engine.configure_stub_area(crate::stub_area::AreaType::stub(10)).unwrap();
        let stub_options = engine.get_area_options();
        assert!(!stub_options.get_e_bit());
        assert!(!stub_options.get_o_bit());
        assert!(!stub_options.get_np_bit());
        
        // Test NSSA area configuration
        engine.configure_stub_area(crate::stub_area::AreaType::nssa(false, 10)).unwrap();
        let nssa_options = engine.get_area_options();
        assert!(!nssa_options.get_e_bit());
        assert!(nssa_options.get_np_bit());
        assert!(!nssa_options.get_o_bit());
        
        // Test multicast support
        assert!(!engine.supports_multicast());
        engine.set_multicast_support(true);
        assert!(engine.supports_multicast());
        
        // Test demand circuit support
        assert!(!engine.supports_demand_circuits());
        engine.set_demand_circuit_support(true);
        assert!(engine.supports_demand_circuits());
        
        // Test Opaque LSA support
        assert!(engine.supports_opaque_lsa());
        engine.set_opaque_lsa_support(false);
        assert!(!engine.supports_opaque_lsa());
    }

    #[test]
    fn test_all_option_bits() {
        let mut options = OSPFOptions::new();
        
        // Test all bits individually
        options.set_mt_bit(true);
        assert!(options.get_mt_bit());
        assert_eq!(options.as_byte() & 0x01, 0x01);
        
        options.set_e_bit(true);
        assert!(options.get_e_bit());
        assert_eq!(options.as_byte() & 0x02, 0x02);
        
        options.set_mc_bit(true);
        assert!(options.get_mc_bit());
        assert_eq!(options.as_byte() & 0x04, 0x04);
        
        options.set_np_bit(true);
        assert!(options.get_np_bit());
        assert_eq!(options.as_byte() & 0x08, 0x08);
        
        options.set_l_bit(true);
        assert!(options.get_l_bit());
        assert_eq!(options.as_byte() & 0x10, 0x10);
        
        options.set_dc_bit(true);
        assert!(options.get_dc_bit());
        assert_eq!(options.as_byte() & 0x20, 0x20);
        
        options.set_o_bit(true);
        assert!(options.get_o_bit());
        assert_eq!(options.as_byte() & 0x40, 0x40);
        
        options.set_dn_bit(true);
        assert!(options.get_dn_bit());
        assert_eq!(options.as_byte() & 0x80, 0x80);
        
        // All bits should be set
        assert_eq!(options.as_byte(), 0xFF);
    }

    #[test]
    fn test_options_from_byte() {
        let options = OSPFOptions::from_byte(0x46); // E-bit, MC-bit, O-bit
        assert!(!options.get_mt_bit());
        assert!(options.get_e_bit());
        assert!(options.get_mc_bit());
        assert!(!options.get_np_bit());
        assert!(!options.get_l_bit());
        assert!(!options.get_dc_bit());
        assert!(options.get_o_bit());
        assert!(!options.get_dn_bit());
    }

    #[test]
    fn test_options_in_simulation() {
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Test that both routers have compatible options
        if let Some(engine1) = sim.get_ospf_engine(r1) {
            if let Some(engine2) = sim.get_ospf_engine(r2) {
                let options1 = engine1.get_area_options();
                let options2 = engine2.get_area_options();
                
                assert!(options1.is_compatible_with(&options2));
                console_log!("R1 options: {}", options1.to_string());
                console_log!("R2 options: {}", options2.to_string());
            }
        }
    }

    #[test]
    fn test_incompatible_area_types() {
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Configure R1 as standard area and R2 as stub area
        if let Some(engine1) = sim.get_ospf_engine_mut(r1) {
            // R1 keeps standard area options (default)
            let options1 = engine1.get_area_options();
            assert!(options1.get_e_bit());
        }
        
        if let Some(engine2) = sim.get_ospf_engine_mut(r2) {
            engine2.configure_stub_area(crate::stub_area::AreaType::stub(10)).unwrap();
            let options2 = engine2.get_area_options();
            assert!(!options2.get_e_bit());
        }
        
        // Verify incompatibility
        if let Some(engine1) = sim.get_ospf_engine(r1) {
            if let Some(engine2) = sim.get_ospf_engine(r2) {
                let options1 = engine1.get_area_options();
                let options2 = engine2.get_area_options();
                
                assert!(!options1.is_compatible_with(&options2));
                console_log!("R1 (standard) options: {}", options1.to_string());
                console_log!("R2 (stub) options: {}", options2.to_string());
            }
        }
    }
}