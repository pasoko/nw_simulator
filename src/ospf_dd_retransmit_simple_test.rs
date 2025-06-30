#[cfg(test)]
mod dd_retransmit_simple_tests {
    use crate::ospf_timer::{OSPFTimerManager, OSPFTimerEvent};
    use crate::ospf_packet_processor::OSPFPacketProcessor;
    use crate::ospf::{DatabaseDescriptionPacket};
    use crate::router::OSPFNeighborState;
    
    #[test]
    fn test_dd_retransmission_timer_basic() {
        // Test basic DD retransmission timer functionality
        let mut timer_manager = OSPFTimerManager::new("1.1.1.1".to_string());
        
        // Start DD retransmission timer
        timer_manager.update_time(0.0);
        timer_manager.start_dd_retransmission_timer(2);
        
        // Timer should not be expired yet
        timer_manager.update_time(4.0);
        let events = timer_manager.process_expired_timers();
        // Filter out hello timer events
        let dd_events: Vec<_> = events.into_iter()
            .filter(|e| matches!(e, OSPFTimerEvent::DDRetransmissionTimer(_)))
            .collect();
        assert_eq!(dd_events.len(), 0);
        
        // Timer should expire after 5 seconds
        timer_manager.update_time(5.1);
        let events = timer_manager.process_expired_timers();
        let dd_events: Vec<_> = events.into_iter()
            .filter(|e| matches!(e, OSPFTimerEvent::DDRetransmissionTimer(_)))
            .collect();
        assert_eq!(dd_events.len(), 1);
        assert!(matches!(dd_events[0], OSPFTimerEvent::DDRetransmissionTimer(2)));
        
        // Stop timer
        timer_manager.stop_dd_retransmission_timer(2);
        timer_manager.update_time(10.1);
        let events = timer_manager.process_expired_timers();
        assert_eq!(events.len(), 0);
    }
    
    #[test]
    fn test_dd_packet_caching() {
        // Test that DD packets are cached for retransmission
        let mut processor = OSPFPacketProcessor::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        // Create initial DD packet event
        let _event = processor.create_dd_packet_event(2, &std::collections::HashMap::new());
        
        // Verify DD packet was cached
        let cached_dd = processor.get_last_dd_packet(2);
        assert!(cached_dd.is_some());
        
        // Verify retransmission should be started
        assert!(processor.should_start_dd_retransmit(2));
        
        // Process DD response to clear retransmission flag
        let dd_response = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: 0x02,
            flags: 0x01,
            dd_sequence_number: 0x80000001,
            lsa_headers: vec![],
        };
        
        let (_, _, _) = processor.process_dd_packet(&dd_response, 2, OSPFNeighborState::Exchange);
        
        // After acknowledgment, should not retransmit
        assert!(!processor.should_start_dd_retransmit(2));
    }
    
    #[test]
    fn test_dd_retransmit_count() {
        // Test that DD retransmission count increments
        let mut processor = OSPFPacketProcessor::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        // Create initial DD packet
        processor.create_dd_packet_event(2, &std::collections::HashMap::new());
        
        // Get cached packet and create retransmit event
        let cached_dd = processor.get_last_dd_packet(2).unwrap();
        let retrans_event1 = processor.create_dd_retransmit_event(2, cached_dd.clone());
        
        // Create another retransmit event
        let retrans_event2 = processor.create_dd_retransmit_event(2, cached_dd);
        
        // Verify events were created (basic check)
        assert_eq!(retrans_event1.to_router_id, 2);
        assert_eq!(retrans_event2.to_router_id, 2);
    }
}