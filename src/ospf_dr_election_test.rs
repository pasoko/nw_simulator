#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::console_log;
    
    #[test]
    fn test_dr_election_with_broadcast_network() {
        console_log!("=== DR/BDR Election Test with Broadcast Network ===");
        
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        let r3 = sim.add_router("R3".to_string(), 50.0, 100.0);
        
        // Connect routers with broadcast network type
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_routers(r1, r3, 10).unwrap();
        
        // Enable OSPF on all routers
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Start simulation
        sim.start_simulation();
        
        // Run simulation for 60 seconds to allow DR election
        let mut time = 0.0;
        while time < 60.0 {
            sim.step_simulation(0.5);
            time += 0.5;
        }
        
        // Verify DR/BDR election occurred
        let mut dr_elected = false;
        let mut bdr_elected = false;
        
        // Check engine states
        for router_id in [r1, r2, r3] {
            if let Some(engine) = sim.get_ospf_engine(router_id) {
                let interfaces = engine.get_dr_election_interfaces();
                for interface_id in interfaces {
                    let (dr, bdr) = engine.get_interface_dr_bdr(interface_id);
                    console_log!("Router {} Interface {}: DR={}, BDR={}", 
                        router_id, interface_id, dr, bdr);
                        
                    if dr != "0.0.0.0" {
                        dr_elected = true;
                    }
                    if bdr != "0.0.0.0" {
                        bdr_elected = true;
                    }
                }
            }
        }
        
        // For Broadcast networks (new default), DR/BDR election should occur
        assert!(dr_elected, "DR should be elected for Broadcast network (default)");
        assert!(bdr_elected, "BDR should be elected for Broadcast network (default)");
        
        console_log!("DR/BDR election test completed successfully");
    }
    
    #[test]
    fn test_dr_election_with_explicit_broadcast() {
        console_log!("=== DR/BDR Election Test with Explicit Broadcast Network ===");
        
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // Connect routers with explicit broadcast type
        let link_id = sim.topology.connect_routers_with_type(
            r1, r2, 10, Some(crate::network_type::OSPFNetworkType::Broadcast)
        ).unwrap();
        
        console_log!("Created link {} between R{} and R{} with Broadcast type", link_id, r1, r2);
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Start simulation
        sim.start_simulation();
        
        // Run simulation
        let mut time = 0.0;
        while time < 60.0 {
            sim.step_simulation(0.5);
            time += 0.5;
        }
        
        // Verify DR/BDR election occurred for broadcast network
        let mut dr_elected = false;
        
        for router_id in [r1, r2] {
            if let Some(engine) = sim.get_ospf_engine(router_id) {
                let interfaces = engine.get_dr_election_interfaces();
                for interface_id in interfaces {
                    let (dr, bdr) = engine.get_interface_dr_bdr(interface_id);
                    console_log!("Router {} Interface {}: DR={}, BDR={}", 
                        router_id, interface_id, dr, bdr);
                        
                    if dr != "0.0.0.0" {
                        dr_elected = true;
                    }
                }
            }
        }
        
        assert!(dr_elected, "DR should be elected for Broadcast network");
        
        console_log!("Explicit broadcast DR/BDR election test completed successfully");
    }
    
    #[test]
    fn test_no_dr_election_with_point_to_multipoint() {
        console_log!("=== No DR/BDR Election Test with Point-to-Multipoint Network ===");
        
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        let r3 = sim.add_router("R3".to_string(), 50.0, 100.0);
        
        // Connect routers with explicit Point-to-Multipoint type
        sim.topology.connect_routers_with_type(
            r1, r2, 10, Some(crate::network_type::OSPFNetworkType::PointToMultipoint)
        ).unwrap();
        sim.topology.connect_routers_with_type(
            r2, r3, 10, Some(crate::network_type::OSPFNetworkType::PointToMultipoint)
        ).unwrap();
        sim.topology.connect_routers_with_type(
            r1, r3, 10, Some(crate::network_type::OSPFNetworkType::PointToMultipoint)
        ).unwrap();
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Start simulation
        sim.start_simulation();
        
        // Run simulation
        let mut time = 0.0;
        while time < 60.0 {
            sim.step_simulation(0.5);
            time += 0.5;
        }
        
        // Verify no DR/BDR election occurred
        let mut dr_elected = false;
        let mut bdr_elected = false;
        
        for router_id in [r1, r2, r3] {
            if let Some(engine) = sim.get_ospf_engine(router_id) {
                let interfaces = engine.get_dr_election_interfaces();
                for interface_id in interfaces {
                    let (dr, bdr) = engine.get_interface_dr_bdr(interface_id);
                    console_log!("Router {} Interface {}: DR={}, BDR={}", 
                        router_id, interface_id, dr, bdr);
                        
                    if dr != "0.0.0.0" {
                        dr_elected = true;
                    }
                    if bdr != "0.0.0.0" {
                        bdr_elected = true;
                    }
                }
            }
        }
        
        assert!(!dr_elected, "DR should not be elected for Point-to-Multipoint network");
        assert!(!bdr_elected, "BDR should not be elected for Point-to-Multipoint network");
        
        console_log!("Point-to-Multipoint no DR/BDR election test completed successfully");
    }
}