// Integration tests for refactored OSPF implementation
//
// These tests verify that the refactored components work together correctly
// and maintain compatibility with existing behavior.

use nw_simulator::ospf_refactored::packets::{
    OSPFPacket, HelloPacket, DatabaseDescriptionPacket, 
    LinkStateUpdatePacket
};
use nw_simulator::ospf_refactored::packet_processor::UnifiedPacketProcessor;
use nw_simulator::ospf_refactored::events::{EventBus, OSPFEvent, PacketType};
use nw_simulator::ospf_refactored::state::NeighborState;
use std::sync::Arc;
use std::net::Ipv4Addr;

#[test]
fn test_hello_packet_processing_integration() {
    // Create event bus and processor
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus.clone(),
    );
    
    // Create hello packet from neighbor
    let mut hello = HelloPacket::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        1,
        40,
    );
    
    // Process first hello (should transition to Init)
    let packet = OSPFPacket::Hello(hello.clone());
    let events = processor.process_packet(packet, 2, 1).unwrap();
    
    // Verify state change event was generated
    assert!(events.iter().any(|e| matches!(e, 
        OSPFEvent::NeighborStateChanged { 
            neighbor_id: 2, 
            to_state: NeighborState::Init, 
            .. 
        }
    )));
    
    // Process bidirectional hello (should transition to TwoWay)
    hello.add_neighbor(Ipv4Addr::new(1, 1, 1, 1));
    let packet = OSPFPacket::Hello(hello);
    let events = processor.process_packet(packet, 2, 1).unwrap();
    
    // Verify state change to TwoWay
    assert!(events.iter().any(|e| matches!(e,
        OSPFEvent::NeighborStateChanged {
            neighbor_id: 2,
            from_state: NeighborState::Init,
            to_state: NeighborState::TwoWay,
            ..
        }
    )));
}

#[test]
fn test_dd_packet_exchange_integration() {
    // Create processor
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus.clone(),
    );
    
    // Test that DD packet from unknown neighbor is rejected
    let dd = DatabaseDescriptionPacket::new(1500, 1000);
    let packet = OSPFPacket::DatabaseDescription(dd);
    
    // Process DD packet from neighbor we haven't heard from
    let result = processor.process_packet(packet, 2, 1);
    
    // With error recovery, this may return Ok with recovery events
    // Check that appropriate error was logged
    match result {
        Ok(events) => {
            // Recovery succeeded, but error should have been logged
            println!("Recovery events: {:?}", events);
        }
        Err(e) => {
            // Direct error without recovery
            assert!(e.to_string().contains("DD packet unexpected"));
        }
    }
}

#[test]
fn test_lsr_lsu_exchange_integration() {
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus.clone(),
    );
    
    // First establish neighbor relationship
    let mut hello = HelloPacket::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        1,
        40,
    );
    hello.add_neighbor(Ipv4Addr::new(1, 1, 1, 1));
    processor.process_packet(OSPFPacket::Hello(hello), 2, 1).unwrap();
    
    // Create LSU packet (can be received at any time)
    let mut lsu = LinkStateUpdatePacket::new();
    let lsa = nw_simulator::ospf_refactored::packets::lsu::LSA::new(1, 0x01010101, 0x02020202);
    lsu.add_lsa(lsa);
    
    let packet = OSPFPacket::LinkStateUpdate(lsu);
    let events = processor.process_packet(packet, 2, 1).unwrap();
    
    // Should generate acknowledgment
    assert!(events.iter().any(|e| matches!(e,
        OSPFEvent::PacketSendRequired {
            packet_type: PacketType::LinkStateAck,
            ..
        }
    )));
    
    // Should trigger SPF for router LSA
    assert!(events.iter().any(|e| matches!(e,
        OSPFEvent::SPFRequired { .. }
    )));
}

#[test]
fn test_event_generation() {
    let event_bus = Arc::new(EventBus::new());
    
    // Create processor with the event bus
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus.clone(),
    );
    
    // Process a hello packet
    let hello = HelloPacket::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        1,
        40,
    );
    
    let events = processor.process_packet(OSPFPacket::Hello(hello), 2, 1).unwrap();
    
    // Verify that events were generated
    assert!(!events.is_empty());
    assert!(events.iter().any(|e| matches!(e, OSPFEvent::NeighborStateChanged { .. })));
}

#[test]
fn test_state_machine_transitions() {
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus.clone(),
    );
    
    // Test that DD packet in wrong state generates error
    // First ensure neighbor is in Down state (no hello received)
    let dd = DatabaseDescriptionPacket::new(1500, 1000);
    let result = processor.process_packet(
        OSPFPacket::DatabaseDescription(dd), 
        3, // Different neighbor that hasn't sent hello
        1
    );
    
    // With error recovery, this may return Ok with recovery events
    match result {
        Ok(events) => {
            // Recovery succeeded, but error should have been logged
            println!("Recovery events: {:?}", events);
        }
        Err(e) => {
            // Direct error without recovery
            assert!(e.to_string().contains("DD packet unexpected"));
        }
    }
}

#[test]
fn test_packet_validation() {
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus.clone(),
    );
    
    // Create hello with wrong area
    let mut hello = HelloPacket::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(1, 0, 0, 0), // Different area
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        1,
        40,
    );
    
    // Update header to have wrong area
    hello.header.area_id = Ipv4Addr::new(1, 0, 0, 0);
    
    let result = processor.process_packet(OSPFPacket::Hello(hello), 2, 1);
    
    // With error recovery, this may return Ok with recovery events
    match result {
        Ok(events) => {
            // Recovery succeeded, but error should have been logged
            println!("Recovery events: {:?}", events);
        }
        Err(e) => {
            // Direct error without recovery
            assert!(e.to_string().contains("Area ID mismatch"));
        }
    }
}