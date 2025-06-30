#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
    use crate::protocol::{ProtocolPacket, PacketEvent};
    
    #[test]
    fn test_area_id_validation() {
        let mut sim = NetworkSimulation::new();
        
        // Create two routers
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // Enable OSPF with different area IDs
        sim.enable_ospf(r1).unwrap(); // Default area 0.0.0.0
        
        // Manually set R2 to different area (this would require adding a method to set area)
        // For now, we'll test that packets with wrong area ID are discarded
        
        // Create a packet with wrong area ID
        let wrong_area_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: "1.1.1.2".to_string(),
            area_id: "0.0.0.1".to_string(), // Wrong area!
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::Hello(HelloPacket {
                network_mask: "255.255.255.0".to_string(),
                hello_interval: 10,
                options: 0x02,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: vec![],
            }),
        };
        
        let event = PacketEvent {
            timestamp: 0.0,
            from_router_id: r2,
            to_router_id: r1,
            packet: ProtocolPacket::OSPF(wrong_area_packet),
        };
        
        // Process the packet - it should be discarded
        sim.protocol_engine.events.push(event);
        sim.step_simulation(0.1);
        
        // Check event log for packet discard
        let events = sim.get_recent_events(10);
        let discard_event = events.iter().any(|e| {
            matches!(&e.event_type, 
                crate::event_manager::SimulationEventType::PacketDiscarded { .. })
        });
        
        assert!(discard_event, "Packet with wrong area ID should be discarded");
    }
}