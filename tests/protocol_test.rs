use nw_simulator::{ProtocolPacket, PacketEvent, ProtocolEngine, RoutingProtocol};
use nw_simulator::{ICMPPacket, ICMPType};
use nw_simulator::{OSPFPacket, OSPFPacketType};
use std::any::Any;

// Mock implementation of RoutingProtocol for testing
struct MockProtocol {
    router_id: u32,
    running: bool,
    current_time: f64,
    packets_to_generate: Vec<PacketEvent>,
}

impl MockProtocol {
    fn new(router_id: u32) -> Self {
        MockProtocol {
            router_id,
            running: false,
            current_time: 0.0,
            packets_to_generate: Vec::new(),
        }
    }

    fn add_packet_to_generate(&mut self, event: PacketEvent) {
        self.packets_to_generate.push(event);
    }
}

impl RoutingProtocol for MockProtocol {
    fn process_packet(&mut self, packet: ProtocolPacket, from_router_id: u32) -> Vec<PacketEvent> {
        // Simple echo behavior for testing
        vec![PacketEvent {
            timestamp: self.current_time + 1.0,
            from_router_id: self.router_id,
            to_router_id: from_router_id,
            packet,
        }]
    }

    fn generate_packets(&mut self, _current_time: f64) -> Vec<PacketEvent> {
        self.packets_to_generate.drain(..).collect()
    }

    fn get_protocol_name(&self) -> &str {
        "MockProtocol"
    }

    fn start(&mut self) {
        self.running = true;
    }

    fn stop(&mut self) {
        self.running = false;
    }

    fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }

    fn get_router_id(&self) -> u32 {
        self.router_id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn create_test_ospf_packet() -> OSPFPacket {
    use nw_simulator::{OSPFPacketData, HelloPacket};
    use nw_simulator::{AuthType, AuthData};
    use nw_simulator::OSPFOptions;
    
    OSPFPacket {
        version: 2,
        packet_type: OSPFPacketType::Hello,
        router_id: "1.1.1.1".to_string(),
        area_id: "0.0.0.0".to_string(),
        checksum: 0,
        auth_type: AuthType::Null,
        auth_data: AuthData::Null,
        data: OSPFPacketData::Hello(HelloPacket {
            network_mask: "255.255.255.0".to_string(),
            hello_interval: 10,
            options: OSPFOptions::default(),
            router_priority: 1,
            router_dead_interval: 40,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: vec![],
        }),
    }
}

fn create_test_icmp_packet() -> ICMPPacket {
    ICMPPacket {
        packet_type: ICMPType::EchoRequest,
        code: 0,
        checksum: 0,
        identifier: 1234,
        sequence_number: 1,
        data: vec![0; 56],
        source_ip: "192.168.1.1".to_string(),
        destination_ip: "192.168.1.2".to_string(),
        ttl: 64,
        original_packet: None,
    }
}

#[test]
fn test_protocol_packet_enum() {
    // Test OSPF variant
    let ospf_packet = create_test_ospf_packet();
    let protocol_packet = ProtocolPacket::OSPF(ospf_packet.clone());
    
    match protocol_packet {
        ProtocolPacket::OSPF(p) => {
            assert_eq!(p.router_id, "1.1.1.1");
            assert_eq!(p.packet_type, OSPFPacketType::Hello);
        },
        _ => panic!("Expected OSPF packet"),
    }
    
    // Test ICMP variant
    let icmp_packet = create_test_icmp_packet();
    let protocol_packet = ProtocolPacket::ICMP(icmp_packet.clone());
    
    match protocol_packet {
        ProtocolPacket::ICMP(p) => {
            assert_eq!(p.source_ip, "192.168.1.1");
            assert_eq!(p.destination_ip, "192.168.1.2");
        },
        _ => panic!("Expected ICMP packet"),
    }
}

#[test]
fn test_packet_event() {
    let packet = ProtocolPacket::ICMP(create_test_icmp_packet());
    let event = PacketEvent {
        timestamp: 10.5,
        from_router_id: 1,
        to_router_id: 2,
        packet,
    };
    
    assert_eq!(event.timestamp, 10.5);
    assert_eq!(event.from_router_id, 1);
    assert_eq!(event.to_router_id, 2);
}

#[test]
fn test_protocol_engine_creation() {
    let engine = ProtocolEngine::new();
    
    assert_eq!(engine.current_time, 0.0);
    assert!(engine.events.is_empty());
}

#[test]
fn test_schedule_event() {
    let mut engine = ProtocolEngine::new();
    
    // Schedule events out of order
    let event1 = PacketEvent {
        timestamp: 5.0,
        from_router_id: 1,
        to_router_id: 2,
        packet: ProtocolPacket::ICMP(create_test_icmp_packet()),
    };
    
    let event2 = PacketEvent {
        timestamp: 2.0,
        from_router_id: 2,
        to_router_id: 3,
        packet: ProtocolPacket::OSPF(create_test_ospf_packet()),
    };
    
    let event3 = PacketEvent {
        timestamp: 7.0,
        from_router_id: 3,
        to_router_id: 1,
        packet: ProtocolPacket::ICMP(create_test_icmp_packet()),
    };
    
    engine.schedule_event(event1);
    engine.schedule_event(event2);
    engine.schedule_event(event3);
    
    // Events should be sorted by timestamp
    assert_eq!(engine.events.len(), 3);
    assert_eq!(engine.events[0].timestamp, 2.0);
    assert_eq!(engine.events[1].timestamp, 5.0);
    assert_eq!(engine.events[2].timestamp, 7.0);
}

#[test]
fn test_process_next_event() {
    let mut engine = ProtocolEngine::new();
    
    // Schedule some events
    let event1 = PacketEvent {
        timestamp: 1.0,
        from_router_id: 1,
        to_router_id: 2,
        packet: ProtocolPacket::ICMP(create_test_icmp_packet()),
    };
    
    let event2 = PacketEvent {
        timestamp: 2.0,
        from_router_id: 2,
        to_router_id: 3,
        packet: ProtocolPacket::OSPF(create_test_ospf_packet()),
    };
    
    engine.schedule_event(event1);
    engine.schedule_event(event2);
    
    // Process first event
    let processed1 = engine.process_next_event();
    assert!(processed1.is_some());
    let event = processed1.unwrap();
    assert_eq!(event.timestamp, 1.0);
    assert_eq!(engine.current_time, 1.0);
    assert_eq!(engine.events.len(), 1);
    
    // Process second event
    let processed2 = engine.process_next_event();
    assert!(processed2.is_some());
    let event = processed2.unwrap();
    assert_eq!(event.timestamp, 2.0);
    assert_eq!(engine.current_time, 2.0);
    assert!(engine.events.is_empty());
    
    // No more events
    let processed3 = engine.process_next_event();
    assert!(processed3.is_none());
}

#[test]
fn test_mock_protocol_implementation() {
    let mut protocol = MockProtocol::new(1);
    
    // Test initial state
    assert_eq!(protocol.get_router_id(), 1);
    assert_eq!(protocol.get_protocol_name(), "MockProtocol");
    assert!(!protocol.running);
    
    // Test start/stop
    protocol.start();
    assert!(protocol.running);
    protocol.stop();
    assert!(!protocol.running);
    
    // Test time update
    protocol.update_time(10.0);
    assert_eq!(protocol.current_time, 10.0);
    
    // Test packet processing
    let packet = ProtocolPacket::ICMP(create_test_icmp_packet());
    let events = protocol.process_packet(packet.clone(), 2);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].timestamp, 11.0); // current_time + 1.0
    assert_eq!(events[0].from_router_id, 1);
    assert_eq!(events[0].to_router_id, 2);
    
    // Test packet generation
    let test_event = PacketEvent {
        timestamp: 15.0,
        from_router_id: 1,
        to_router_id: 3,
        packet: ProtocolPacket::OSPF(create_test_ospf_packet()),
    };
    protocol.add_packet_to_generate(test_event);
    let generated = protocol.generate_packets(15.0);
    assert_eq!(generated.len(), 1);
    assert_eq!(generated[0].to_router_id, 3);
}

#[test]
fn test_any_cast() {
    let mut protocol = MockProtocol::new(1);
    
    // Test as_any
    let any_ref = protocol.as_any();
    assert!(any_ref.downcast_ref::<MockProtocol>().is_some());
    
    // Test as_any_mut
    let any_mut = protocol.as_any_mut();
    if let Some(mock) = any_mut.downcast_mut::<MockProtocol>() {
        mock.router_id = 2;
    }
    assert_eq!(protocol.router_id, 2);
}

#[test]
fn test_protocol_packet_serialization() {
    // Test OSPF packet serialization
    let ospf_packet = create_test_ospf_packet();
    let protocol_packet = ProtocolPacket::OSPF(ospf_packet);
    
    let json = serde_json::to_string(&protocol_packet).unwrap();
    let deserialized: ProtocolPacket = serde_json::from_str(&json).unwrap();
    
    match deserialized {
        ProtocolPacket::OSPF(p) => {
            assert_eq!(p.router_id, "1.1.1.1");
            assert_eq!(p.packet_type, OSPFPacketType::Hello);
        },
        _ => panic!("Expected OSPF packet"),
    }
    
    // Test ICMP packet serialization
    let icmp_packet = create_test_icmp_packet();
    let protocol_packet = ProtocolPacket::ICMP(icmp_packet);
    
    let json = serde_json::to_string(&protocol_packet).unwrap();
    let deserialized: ProtocolPacket = serde_json::from_str(&json).unwrap();
    
    match deserialized {
        ProtocolPacket::ICMP(p) => {
            assert_eq!(p.source_ip, "192.168.1.1");
            assert_eq!(p.destination_ip, "192.168.1.2");
        },
        _ => panic!("Expected ICMP packet"),
    }
}

#[test]
fn test_packet_event_serialization() {
    let packet = ProtocolPacket::ICMP(create_test_icmp_packet());
    let event = PacketEvent {
        timestamp: 10.5,
        from_router_id: 1,
        to_router_id: 2,
        packet,
    };
    
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: PacketEvent = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.timestamp, 10.5);
    assert_eq!(deserialized.from_router_id, 1);
    assert_eq!(deserialized.to_router_id, 2);
}

#[test]
fn test_protocol_engine_with_same_timestamps() {
    let mut engine = ProtocolEngine::new();
    
    // Schedule events with same timestamp
    for i in 0..3 {
        let event = PacketEvent {
            timestamp: 5.0,
            from_router_id: i,
            to_router_id: i + 1,
            packet: ProtocolPacket::ICMP(create_test_icmp_packet()),
        };
        engine.schedule_event(event);
    }
    
    assert_eq!(engine.events.len(), 3);
    
    // All should have same timestamp
    for event in &engine.events {
        assert_eq!(event.timestamp, 5.0);
    }
    
    // Process all events
    let mut processed_ids = vec![];
    for _ in 0..3 {
        let event = engine.process_next_event().unwrap();
        processed_ids.push(event.from_router_id);
        assert_eq!(engine.current_time, 5.0);
    }
    
    // Check that all IDs were processed (order may vary)
    processed_ids.sort();
    assert_eq!(processed_ids, vec![0, 1, 2]);
}