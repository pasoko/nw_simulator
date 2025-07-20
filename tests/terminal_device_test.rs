use nw_simulator::terminal_device::*;
use nw_simulator::device::{ICMPPacket, ICMPType};

fn create_test_terminal(id: u32) -> TerminalDevice {
    TerminalDevice::new(
        id,
        format!("Terminal-{}", id),
        format!("192.168.{}.100", id),
        "255.255.255.0".to_string(),
        format!("192.168.{}.1", id),
    )
}

#[test]
fn test_terminal_device_creation() {
    let terminal = create_test_terminal(1);
    
    assert_eq!(terminal.id, 1);
    assert_eq!(terminal.name, "Terminal-1");
    assert_eq!(terminal.ip_address, "192.168.1.100");
    assert_eq!(terminal.netmask, "255.255.255.0");
    assert_eq!(terminal.default_gateway, "192.168.1.1");
    assert_eq!(terminal.mac_address, "00:00:00:00:00:01");
    assert_eq!(terminal.connected_router_id, None);
    assert_eq!(terminal.connected_interface_id, None);
    assert!(!terminal.is_failed);
    assert!(terminal.arp_table.is_empty());
    assert_eq!(terminal.routing_table.len(), 1); // Default route
    assert!(terminal.packet_queue.is_empty());
}

#[test]
fn test_connect_to_router() {
    let mut terminal = create_test_terminal(1);
    
    terminal.connect_to_router(10, 2);
    
    assert_eq!(terminal.connected_router_id, Some(10));
    assert_eq!(terminal.connected_interface_id, Some(2));
}

#[test]
fn test_disconnect_from_router() {
    let mut terminal = create_test_terminal(1);
    terminal.connect_to_router(10, 2);
    
    terminal.disconnect();
    
    assert_eq!(terminal.connected_router_id, None);
    assert_eq!(terminal.connected_interface_id, None);
}

#[test]
fn test_is_in_same_subnet() {
    let terminal = create_test_terminal(1);
    
    assert!(terminal.is_in_same_subnet("192.168.1.50"));
    assert!(terminal.is_in_same_subnet("192.168.1.254"));
    assert!(!terminal.is_in_same_subnet("192.168.2.100"));
    assert!(!terminal.is_in_same_subnet("10.0.0.1"));
}

#[test]
fn test_arp_table_operations() {
    let mut terminal = create_test_terminal(1);
    
    // Add ARP entry
    terminal.add_arp_entry("192.168.1.1", "aa:bb:cc:dd:ee:ff");
    assert_eq!(terminal.arp_table.get("192.168.1.1"), Some(&"aa:bb:cc:dd:ee:ff".to_string()));
    assert_eq!(terminal.statistics.arp_replies_received, 1);
    
    // Get MAC address
    assert_eq!(terminal.get_mac_for_ip("192.168.1.1"), Some("aa:bb:cc:dd:ee:ff".to_string()));
    assert_eq!(terminal.get_mac_for_ip("192.168.1.2"), None);
}

#[test]
fn test_route_lookup() {
    let mut terminal = create_test_terminal(1);
    
    // Add specific route
    terminal.add_route("10.0.0.0", "255.255.255.0", "192.168.1.1", 10);
    
    // Test route lookup
    let route1 = terminal.lookup_route("10.0.0.5");
    assert!(route1.is_some());
    assert_eq!(route1.unwrap().destination, "10.0.0.0");
    
    // Test default route
    let route2 = terminal.lookup_route("8.8.8.8");
    assert!(route2.is_some());
    assert!(route2.unwrap().is_default);
    
    // Test direct route
    let route3 = terminal.lookup_route("192.168.1.50");
    assert!(route3.is_some());
    assert_eq!(route3.unwrap().gateway, "direct");
}

#[test]
fn test_can_reach() {
    let mut terminal = create_test_terminal(1);
    terminal.connect_to_router(10, 2);
    
    // Can reach when connected and not failed
    assert!(terminal.can_reach("8.8.8.8"));
    
    // Cannot reach when disconnected
    terminal.disconnect();
    assert!(!terminal.can_reach("8.8.8.8"));
    
    // Cannot reach when failed
    terminal.connect_to_router(10, 2);
    terminal.set_failed(true);
    assert!(!terminal.can_reach("8.8.8.8"));
    
    // Can reach same subnet even when disconnected
    terminal.disconnect();
    terminal.set_failed(false);
    assert!(terminal.can_reach("192.168.1.50"));
}

#[test]
fn test_create_echo_request() {
    let terminal = create_test_terminal(1);
    
    let packet = terminal.create_echo_request("10.0.0.1", 64, 1234, 5678);
    
    assert_eq!(packet.source_ip, "192.168.1.100");
    assert_eq!(packet.destination_ip, "10.0.0.1");
    assert_eq!(packet.packet_type, ICMPType::EchoRequest);
    assert_eq!(packet.ttl, 64);
    assert_eq!(packet.identifier, 1234);
    assert_eq!(packet.sequence_number, 5678);
    assert_eq!(packet.data.len(), 56); // Default ping data size
}

#[test]
fn test_process_echo_request() {
    let mut terminal = create_test_terminal(1);
    
    let request = ICMPPacket {
        packet_type: ICMPType::EchoRequest,
        code: 0,
        checksum: 0,
        identifier: 1234,
        sequence_number: 5678,
        data: vec![0; 56],
        source_ip: "192.168.1.50".to_string(),
        destination_ip: "192.168.1.100".to_string(),
        ttl: 64,
        original_packet: None,
    };
    
    let reply = terminal.process_echo_request(&request);
    assert!(reply.is_some());
    
    let reply_packet = reply.unwrap();
    assert_eq!(reply_packet.source_ip, "192.168.1.100");
    assert_eq!(reply_packet.destination_ip, "192.168.1.50");
    assert_eq!(reply_packet.packet_type, ICMPType::EchoReply);
    assert_eq!(reply_packet.identifier, 1234);
    assert_eq!(reply_packet.sequence_number, 5678);
    assert_eq!(terminal.statistics.icmp_echo_replies_sent, 1);
    
    // Test request to different IP (should return None)
    let request2 = ICMPPacket {
        packet_type: ICMPType::EchoRequest,
        code: 0,
        checksum: 0,
        identifier: 1234,
        sequence_number: 5678,
        data: vec![0; 56],
        source_ip: "192.168.1.50".to_string(),
        destination_ip: "192.168.1.101".to_string(), // Different IP
        ttl: 64,
        original_packet: None,
    };
    
    let reply2 = terminal.process_echo_request(&request2);
    assert!(reply2.is_none());
}

#[test]
fn test_receive_echo_reply() {
    let mut terminal = create_test_terminal(1);
    
    let reply = ICMPPacket {
        packet_type: ICMPType::EchoReply,
        code: 0,
        checksum: 0,
        identifier: 1234,
        sequence_number: 5678,
        data: vec![0; 56],
        source_ip: "10.0.0.1".to_string(),
        destination_ip: "192.168.1.100".to_string(),
        ttl: 63,
        original_packet: None,
    };
    
    terminal.receive_echo_reply(&reply);
    assert_eq!(terminal.statistics.icmp_echo_replies_received, 1);
}

#[test]
fn test_queue_packet() {
    let mut terminal = create_test_terminal(1);
    
    let packet = terminal.create_echo_request("10.0.0.1", 64, 1234, 5678);
    
    // Queue packet
    assert!(terminal.queue_packet(packet.clone(), "10.0.0.1"));
    assert_eq!(terminal.packet_queue.len(), 1);
    
    // Test max queue size
    terminal.config.max_queue_size = 2;
    terminal.queue_packet(packet.clone(), "10.0.0.2");
    assert_eq!(terminal.packet_queue.len(), 2);
    
    // Queue should not accept more when full
    assert!(!terminal.queue_packet(packet, "10.0.0.3"));
    assert_eq!(terminal.packet_queue.len(), 2);
    assert_eq!(terminal.statistics.packets_dropped, 1);
}

#[test]
fn test_process_packet_queue() {
    let mut terminal = create_test_terminal(1);
    terminal.connect_to_router(10, 2);
    terminal.add_arp_entry("192.168.1.1", "aa:bb:cc:dd:ee:ff");
    
    // Queue some packets
    let packet1 = terminal.create_echo_request("10.0.0.1", 64, 1, 1);
    let packet2 = terminal.create_echo_request("192.168.1.50", 64, 2, 1);
    
    terminal.queue_packet(packet1, "10.0.0.1");
    terminal.queue_packet(packet2, "192.168.1.50");
    
    // Process queue
    let sent_packets = terminal.process_packet_queue(100.0);
    assert_eq!(sent_packets.len(), 2);
    assert_eq!(terminal.packet_queue.len(), 0);
    assert_eq!(terminal.statistics.packets_sent, 2);
    
    // Test retry mechanism
    terminal.config.max_retries = 2;
    terminal.config.retry_interval = 1.0;
    terminal.disconnect(); // Disconnect to prevent sending
    
    let packet3 = terminal.create_echo_request("10.0.0.1", 64, 3, 1);
    terminal.queue_packet(packet3, "10.0.0.1");
    
    // First attempt - should fail and remain in queue
    let sent = terminal.process_packet_queue(100.0);
    assert_eq!(sent.len(), 0);
    assert_eq!(terminal.packet_queue.len(), 1);
    assert_eq!(terminal.packet_queue[0].retry_count, 1);
    
    // Second attempt after retry interval
    let sent = terminal.process_packet_queue(101.5);
    assert_eq!(sent.len(), 0);
    assert_eq!(terminal.packet_queue.len(), 1);
    assert_eq!(terminal.packet_queue[0].retry_count, 2);
    
    // Third attempt - should drop packet
    let sent = terminal.process_packet_queue(102.5);
    assert_eq!(sent.len(), 0);
    assert_eq!(terminal.packet_queue.len(), 0);
    assert_eq!(terminal.statistics.packets_dropped, 2); // 1 from previous test + 1
}

#[test]
fn test_clear_expired_packets() {
    let mut terminal = create_test_terminal(1);
    terminal.config.packet_timeout = 10.0;
    
    // Queue packets at different times
    let packet1 = terminal.create_echo_request("10.0.0.1", 64, 1, 1);
    let packet2 = terminal.create_echo_request("10.0.0.2", 64, 2, 1);
    
    terminal.packet_queue.push_back(QueuedPacket {
        packet: packet1,
        destination_ip: "10.0.0.1".to_string(),
        retry_count: 0,
        next_retry_time: 0.0,
        creation_time: 0.0,
    });
    
    terminal.packet_queue.push_back(QueuedPacket {
        packet: packet2,
        destination_ip: "10.0.0.2".to_string(),
        retry_count: 0,
        next_retry_time: 0.0,
        creation_time: 5.0,
    });
    
    // Clear expired packets
    terminal.clear_expired_packets(11.0);
    assert_eq!(terminal.packet_queue.len(), 1);
    assert_eq!(terminal.statistics.packets_dropped, 1);
    
    terminal.clear_expired_packets(16.0);
    assert_eq!(terminal.packet_queue.len(), 0);
    assert_eq!(terminal.statistics.packets_dropped, 2);
}

#[test]
fn test_reset_statistics() {
    let mut terminal = create_test_terminal(1);
    
    // Modify statistics
    terminal.statistics.packets_sent = 100;
    terminal.statistics.packets_received = 50;
    terminal.statistics.packets_dropped = 10;
    
    terminal.reset_statistics();
    
    assert_eq!(terminal.statistics.packets_sent, 0);
    assert_eq!(terminal.statistics.packets_received, 0);
    assert_eq!(terminal.statistics.packets_dropped, 0);
    assert_eq!(terminal.statistics.icmp_echo_requests_sent, 0);
    assert_eq!(terminal.statistics.icmp_echo_replies_received, 0);
}

#[test]
fn test_ip_to_u32_conversions() {
    assert_eq!(ip_to_u32("192.168.1.100"), 3232235876);
    assert_eq!(ip_to_u32("10.0.0.1"), 167772161);
    assert_eq!(ip_to_u32("255.255.255.255"), 4294967295);
    assert_eq!(ip_to_u32("0.0.0.0"), 0);
    
    // Test invalid IP
    assert_eq!(ip_to_u32("invalid"), 0);
    assert_eq!(ip_to_u32("256.0.0.1"), 0);
}

#[test]
fn test_subnet_calculations() {
    let terminal = create_test_terminal(1);
    
    // Test same subnet (192.168.1.0/24)
    assert!(terminal.is_in_same_subnet("192.168.1.1"));
    assert!(terminal.is_in_same_subnet("192.168.1.254"));
    assert!(!terminal.is_in_same_subnet("192.168.2.1"));
    
    // Test with different netmask
    let mut terminal2 = create_test_terminal(2);
    terminal2.netmask = "255.255.0.0".to_string();
    terminal2.ip_address = "10.0.0.100".to_string();
    
    assert!(terminal2.is_in_same_subnet("10.0.255.254"));
    assert!(!terminal2.is_in_same_subnet("10.1.0.1"));
}

#[test]
fn test_multiple_routes() {
    let mut terminal = create_test_terminal(1);
    
    // Add multiple routes
    terminal.add_route("10.0.0.0", "255.255.255.0", "192.168.1.1", 10);
    terminal.add_route("10.0.1.0", "255.255.255.0", "192.168.1.2", 20);
    terminal.add_route("10.0.0.0", "255.255.0.0", "192.168.1.3", 30);
    
    // Lookup should return most specific route
    let route = terminal.lookup_route("10.0.0.5");
    assert!(route.is_some());
    assert_eq!(route.unwrap().gateway, "192.168.1.1");
    assert_eq!(route.unwrap().metric, 10);
    
    // Test less specific match
    let route2 = terminal.lookup_route("10.0.2.5");
    assert!(route2.is_some());
    assert_eq!(route2.unwrap().gateway, "192.168.1.3");
    assert_eq!(route2.unwrap().metric, 30);
}