#[cfg(test)]
mod area_tests {
    use crate::simulation::NetworkSimulation;
    use crate::event_manager::SimulationEventType;
    
    #[test]
    fn test_area_id_validation() {
        let mut sim = NetworkSimulation::new();
        
        // Create routers in different areas
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        
        // Enable OSPF on all routers
        // By default, all are in area 0.0.0.0
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Run simulation to establish adjacencies
        sim.start_simulation();
        
        // Step through multiple times to allow hello exchange
        for _ in 0..20 {
            sim.step_simulation(1.0);
        }
        
        // Verify no packets were discarded (all in same area)
        let events = sim.get_recent_events(1000);
        let discarded_count = events.iter()
            .filter(|e| matches!(&e.event_type, SimulationEventType::PacketDiscarded { .. }))
            .count();
        assert_eq!(discarded_count, 0, "No packets should be discarded when all routers are in the same area");
        
        // Check that neighbor discovery started
        let neighbor_events = events.iter()
            .filter(|e| matches!(&e.event_type, SimulationEventType::NeighborStateChanged { .. }))
            .count();
        assert!(neighbor_events > 0, "Neighbor state changes should have occurred");
        
        // Allow more time if needed
        if sim.get_ospf_neighbor_count(r1) == 0 {
            for _ in 0..20 {
                sim.step_simulation(1.0);
            }
        }
        
        // Verify neighbors were eventually formed  
        assert!(sim.get_ospf_neighbor_count(r1) > 0, "R1 should have neighbors");
        assert!(sim.get_ospf_neighbor_count(r2) > 0, "R2 should have neighbors");
    }
    
    #[test] 
    fn test_area_id_mismatch_packet_drop() {
        // Test that packets from different areas are properly discarded
        // This requires manual packet injection since we can't easily change area IDs after engine creation
        
        let mut sim = NetworkSimulation::new();
        
        // Create routers
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        // Connect and enable OSPF
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Create a packet with mismatched area ID manually
        use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
        use crate::protocol::{ProtocolPacket, PacketEvent};
        
        let mismatched_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: "1.1.1.1".to_string(),
            area_id: "1.1.1.1".to_string(), // Different area!
            checksum: 0,
            auth_type: crate::ospf_auth::AuthType::Null,
            auth_data: crate::ospf_auth::AuthData::None,
            data: OSPFPacketData::Hello(HelloPacket {
                network_mask: "255.255.255.252".to_string(),
                hello_interval: 10,
                options: 0x02,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: vec![],
            }),
        };
        
        // Inject the mismatched packet
        let event = PacketEvent {
            timestamp: 0.1,
            from_router_id: r1,
            to_router_id: r2,
            packet: ProtocolPacket::OSPF(mismatched_packet),
        };
        
        // Start simulation if not already running
        if !sim.running {
            sim.start_simulation();
        }
        
        sim.protocol_engine.schedule_event(event);
        
        // Process the packet - step to the right time
        sim.step_simulation(0.15);  // This should bring us to time 0.15, past the event at 0.1
        
        // Check that packet was discarded
        let events = sim.get_recent_events(100);
        
        let discarded = events.iter()
            .find(|e| matches!(&e.event_type, SimulationEventType::PacketDiscarded { .. }));
        
        assert!(discarded.is_some(), "Packet with mismatched area ID should be discarded");
        
        if let Some(event) = discarded {
            if let SimulationEventType::PacketDiscarded { reason, .. } = &event.event_type {
                assert!(reason.contains("Area ID mismatch"), "Discard reason should mention Area ID mismatch");
            }
        }
    }
}