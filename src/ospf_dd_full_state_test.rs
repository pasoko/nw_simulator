#[cfg(test)]
mod dd_full_state_tests {
    use crate::ospf_engine::OSPFEngine;
    use crate::ospf::{HelloPacket, DatabaseDescriptionPacket, LinkStateUpdatePacket};
    use crate::ospf_options::OSPFOptions;
    use crate::console_log;
    
    #[test]
    fn test_dd_timer_stops_at_full_state() {
        let mut engine1 = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        // Configure router links
        engine1.add_router_link(2, 1, 10);
        
        // Exchange Hello packets to reach TwoWay state
        let hello1 = HelloPacket {
            network_mask: "255.255.255.0".to_string(),
            hello_interval: 10,
            options: OSPFOptions::new(),
            router_priority: 1,
            router_dead_interval: 40,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: vec![],
        };
        
        // Process Hello to reach Init state
        let _events1 = engine1.process_hello_packet(&hello1, 2, 2);
        assert_eq!(engine1.get_neighbor_count(), 1);
        
        // Process Hello with neighbor ID to reach TwoWay/ExStart
        let hello2 = HelloPacket {
            network_mask: "255.255.255.0".to_string(),
            hello_interval: 10,
            options: OSPFOptions::new(),
            router_priority: 1,
            router_dead_interval: 40,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: vec!["1.1.1.1".to_string()],
        };
        
        let _events2 = engine1.process_hello_packet(&hello2, 2, 2);
        
        // Process DD packet to move to Exchange state
        let dd_packet = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: OSPFOptions::new(),
            flags: 0x07, // I, M, MS bits set
            dd_sequence_number: 2147483649,
            lsa_headers: vec![],
        };
        
        let _events3 = engine1.process_dd_packet(&dd_packet, 2);
        
        // Move to Loading state by sending DD with LSA headers
        let dd_packet2 = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: OSPFOptions::new(),
            flags: 0x00, // No more data
            dd_sequence_number: 2147483649,
            lsa_headers: vec![],
        };
        
        let _events4 = engine1.process_dd_packet(&dd_packet2, 2);
        
        // Create an LSU to transition to Full state
        let lsu = LinkStateUpdatePacket {
            lsas: vec![],
        };
        
        // Process LSU to reach Full state
        let _events5 = engine1.process_lsu_packet(&lsu, 2);
        
        // Now simulate time passing to check DD retransmission
        // Move time forward by 10 seconds
        engine1.update_time(10.0);
        
        // DD retransmission timer should have been stopped
        // Check by advancing time again
        let events_at_15s = engine1.update_time(15.0);
        
        // Verify no DD retransmission events
        let dd_retrans_count = events_at_15s.iter()
            .filter(|e| {
                match &e.packet {
                    crate::protocol::ProtocolPacket::OSPF(ospf_packet) => {
                        matches!(ospf_packet.packet_type, crate::ospf::OSPFPacketType::DatabaseDescription)
                    },
                    _ => false,
                }
            })
            .count();
        
        assert_eq!(dd_retrans_count, 0, "No DD retransmission should occur in Full state");
        
        console_log!("Test passed: DD timer correctly stopped in Full state");
    }
}