#[cfg(test)]
mod dd_retransmit_tests {
    use crate::ospf_engine::OSPFEngine;
    use crate::ospf::{HelloPacket, DatabaseDescriptionPacket, OSPFPacketData};
    use crate::protocol::ProtocolPacket;
    
    #[test]
    fn test_dd_retransmission_timer() {
        // Create two OSPF engines
        let mut engine1 = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        let mut engine2 = OSPFEngine::new("1.1.1.2".to_string(), "0.0.0.0".to_string());
        
        // Configure links
        engine1.add_router_link(2, 1, 10);
        engine2.add_router_link(1, 1, 10);
        
        // Process hello packets to establish two-way communication
        let hello1 = HelloPacket {
            network_mask: "255.255.255.252".to_string(),
            hello_interval: 10,
            options: 0x02,
            router_priority: 1,
            router_dead_interval: 40,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: vec![],
        };
        
        // Router 1 receives hello from Router 2
        let events1 = engine1.process_hello_packet(&hello1, 2, 1);
        assert_eq!(events1.len(), 0); // No response yet (state = Init)
        
        // Update time to allow timers to settle
        engine1.update_time(0.0);
        engine2.update_time(0.0);
        
        // Router 1 should respond with hello containing neighbor
        let hello_with_neighbor = HelloPacket {
            network_mask: "255.255.255.252".to_string(),
            hello_interval: 10,
            options: 0x02,
            router_priority: 1,
            router_dead_interval: 40,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: vec!["1.1.1.2".to_string()],
        };
        
        // Router 2 receives hello from Router 1 with neighbor list
        let events2 = engine2.process_hello_packet(&hello_with_neighbor, 1, 1);
        
        // Should move to TwoWay and then ExStart, triggering DD exchange
        let dd_event_count = events2.iter().filter(|e| {
            matches!(&e.packet, ProtocolPacket::OSPF(p) if matches!(&p.data, OSPFPacketData::DatabaseDescription(_)))
        }).count();
        assert!(dd_event_count > 0, "Should trigger DD exchange, got {} events", events2.len());
        
        // Check that DD packet was sent
        let dd_event = events2.iter().find(|e| {
            matches!(&e.packet, ProtocolPacket::OSPF(p) if matches!(&p.data, OSPFPacketData::DatabaseDescription(_)))
        });
        assert!(dd_event.is_some());
        
        // Simulate time passing without DD acknowledgment (5 seconds)
        engine2.update_time(5.0);
        let timer_events = engine2.update_time(5.1);
        
        // Should trigger DD retransmission
        let dd_retrans_event = timer_events.iter().find(|e| {
            matches!(&e.packet, ProtocolPacket::OSPF(p) if matches!(&p.data, OSPFPacketData::DatabaseDescription(_)))
        });
        assert!(dd_retrans_event.is_some(), "DD retransmission timer should have fired");
        
        // Verify DD packet is retransmitted
        if let Some(event) = dd_retrans_event {
            assert_eq!(event.from_router_id, 2);
            assert_eq!(event.to_router_id, 1);
        }
        
        // Now simulate receiving DD response
        let dd_response = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: 0x02,
            flags: 0x01, // MS bit set (master)
            dd_sequence_number: 0x80000001,
            lsa_headers: vec![],
        };
        
        let _response_events = engine2.process_dd_packet(&dd_response, 1);
        
        // Simulate more time passing - should NOT retransmit again
        engine2.update_time(10.0);
        let no_retrans_events = engine2.update_time(10.1);
        
        let dd_event_after_ack = no_retrans_events.iter().find(|e| {
            matches!(&e.packet, ProtocolPacket::OSPF(p) if matches!(&p.data, OSPFPacketData::DatabaseDescription(_)))
        });
        assert!(dd_event_after_ack.is_none(), "DD should not be retransmitted after acknowledgment");
    }
    
    #[test]
    fn test_dd_retransmit_max_count() {
        let mut engine = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        engine.add_router_link(2, 1, 10);
        
        // Process hello to establish neighbor
        let hello = HelloPacket {
            network_mask: "255.255.255.252".to_string(),
            hello_interval: 10,
            options: 0x02,
            router_priority: 1,
            router_dead_interval: 40,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: vec!["1.1.1.1".to_string()],
        };
        
        engine.process_hello_packet(&hello, 2, 1);
        
        // Simulate multiple retransmissions
        let mut retrans_count = 0;
        for i in 1..20 {
            engine.update_time(i as f64 * 5.0);
            let events = engine.update_time(i as f64 * 5.0 + 0.1);
            
            let has_dd = events.iter().any(|e| {
                matches!(&e.packet, ProtocolPacket::OSPF(p) if matches!(&p.data, OSPFPacketData::DatabaseDescription(_)))
            });
            
            if has_dd {
                retrans_count += 1;
            }
        }
        
        // Should have multiple retransmissions
        assert!(retrans_count > 0, "Should have DD retransmissions");
        
        // Verify neighbor doesn't stay in Exchange state forever
        let neighbor_count = engine.get_neighbor_count();
        assert!(neighbor_count <= 1, "Neighbor management should handle excessive retransmissions");
    }
}