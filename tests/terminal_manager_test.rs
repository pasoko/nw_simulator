use nw_simulator::{TerminalManager, ManagerConfig};
use nw_simulator::{ICMPPacket, ICMPType};

fn create_test_manager() -> TerminalManager {
    TerminalManager::new()
}

fn create_test_packet(source: &str, dest: &str) -> ICMPPacket {
    ICMPPacket {
        packet_type: ICMPType::EchoRequest,
        code: 0,
        checksum: 0,
        identifier: 1234,
        sequence_number: 1,
        data: vec![0; 56],
        source_ip: source.to_string(),
        destination_ip: dest.to_string(),
        ttl: 64,
        original_packet: None,
    }
}

#[test]
fn test_terminal_manager_creation() {
    let manager = create_test_manager();
    
    let stats = manager.get_statistics();
    assert_eq!(stats.total_terminals, 0);
    assert_eq!(stats.active_terminals, 0);
    assert_eq!(stats.failed_terminals, 0);
}

#[test]
fn test_add_terminal() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    
    assert!(id.is_ok());
    let terminal_id = id.unwrap();
    assert_eq!(manager.get_statistics().total_terminals, 1);
    assert_eq!(manager.get_statistics().active_terminals, 1);
    
    // Test getting terminal
    let terminal_info = manager.get_terminal_info(terminal_id);
    assert!(terminal_info.is_ok());
    let info = terminal_info.unwrap();
    assert_eq!(info.name, "Terminal-1");
    assert_eq!(info.ip_address, "192.168.1.100");
}

#[test]
fn test_max_terminals_limit() {
    let config = ManagerConfig {
        max_terminals: 2,
        ..Default::default()
    };
    let mut manager = TerminalManager::with_config(config);
    
    // Add two terminals
    let id1 = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    
    let id2 = manager.add_terminal(
        "Terminal-2".to_string(),
        "192.168.1.101".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    
    assert!(id1.is_ok());
    assert!(id2.is_ok());
    
    // Try to add third terminal
    let id3 = manager.add_terminal(
        "Terminal-3".to_string(),
        "192.168.1.102".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    
    assert!(id3.is_err());
    assert_eq!(manager.get_statistics().total_terminals, 2);
}

#[test]
fn test_remove_terminal() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    assert!(id.is_ok());
    let terminal_id = id.unwrap();
    
    assert_eq!(manager.get_statistics().total_terminals, 1);
    
    let result = manager.remove_terminal(terminal_id);
    assert!(result.is_ok());
    
    assert_eq!(manager.get_statistics().total_terminals, 0);
    assert!(manager.get_terminal_info(terminal_id).is_err());
    assert_eq!(manager.get_statistics().total_terminals, 0);
}

#[test]
fn test_connect_terminal_to_router() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    assert!(id.is_ok());
    let terminal_id = id.unwrap();
    
    let result = manager.connect_terminal_to_router(terminal_id, 10, 2);
    assert!(result.is_ok());
    
    let terminal_info = manager.get_terminal_info(terminal_id).unwrap();
    assert_eq!(terminal_info.connected_router_id, Some(10));
    // Note: TerminalDeviceInfo doesn't track the interface ID
}

#[test]
fn test_disconnect_terminal() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    assert!(id.is_ok());
    let terminal_id = id.unwrap();
    
    manager.connect_terminal_to_router(terminal_id, 10, 2).unwrap();
    let result = manager.disconnect_terminal(terminal_id);
    assert!(result.is_ok());
    
    let terminal_info = manager.get_terminal_info(terminal_id).unwrap();
    assert_eq!(terminal_info.connected_router_id, None);
}

#[test]
fn test_set_terminal_failed() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    assert!(id.is_ok());
    let terminal_id = id.unwrap();
    
    manager.update_statistics(0.0);
    assert_eq!(manager.get_statistics().failed_terminals, 0);
    
    let result = manager.set_terminal_failed(terminal_id, true);
    assert!(result.is_ok());
    
    manager.update_statistics(100.0);
    let terminal_info = manager.get_terminal_info(terminal_id).unwrap();
    assert!(terminal_info.is_failed);
    assert_eq!(manager.get_statistics().failed_terminals, 1);
    assert_eq!(manager.get_statistics().active_terminals, 0);
    
    manager.set_terminal_failed(terminal_id, false).unwrap();
    manager.update_statistics(200.0);
    assert_eq!(manager.get_statistics().failed_terminals, 0);
    assert_eq!(manager.get_statistics().active_terminals, 1);
}

#[test]
fn test_get_all_terminals_info() {
    let mut manager = create_test_manager();
    
    let id1 = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    assert!(id1.is_ok());
    let terminal_id1 = id1.unwrap();
    
    let id2 = manager.add_terminal(
        "Terminal-2".to_string(),
        "192.168.1.101".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    );
    assert!(id2.is_ok());
    let terminal_id2 = id2.unwrap();
    
    let terminals = manager.get_all_terminals_info();
    assert_eq!(terminals.len(), 2);
    
    let mut ids: Vec<u32> = terminals.iter().map(|t| t.id).collect();
    ids.sort();
    assert_eq!(ids, vec![terminal_id1, terminal_id2]);
}

#[test]
fn test_find_terminal_by_ip() {
    let mut manager = create_test_manager();
    
    let id1 = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    let id2 = manager.add_terminal(
        "Terminal-2".to_string(),
        "192.168.1.101".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    let terminal1_id = manager.find_terminal_by_ip("192.168.1.100");
    assert!(terminal1_id.is_some());
    assert_eq!(terminal1_id.unwrap(), id1);
    
    let terminal2_id = manager.find_terminal_by_ip("192.168.1.101");
    assert!(terminal2_id.is_some());
    assert_eq!(terminal2_id.unwrap(), id2);
    
    let terminal3_id = manager.find_terminal_by_ip("192.168.1.102");
    assert!(terminal3_id.is_none());
}

#[test]
fn test_get_terminals_on_router() {
    let mut manager = create_test_manager();
    
    let id1 = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    let id2 = manager.add_terminal(
        "Terminal-2".to_string(),
        "192.168.1.101".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    let id3 = manager.add_terminal(
        "Terminal-3".to_string(),
        "192.168.1.102".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    manager.connect_terminal_to_router(id1, 10, 1).unwrap();
    manager.connect_terminal_to_router(id2, 10, 2).unwrap();
    manager.connect_terminal_to_router(id3, 20, 1).unwrap();
    
    // Check connections via get_all_terminals_info
    let terminals = manager.get_all_terminals_info();
    let terminals_on_router10: Vec<_> = terminals.iter()
        .filter(|t| t.connected_router_id == Some(10))
        .collect();
    assert_eq!(terminals_on_router10.len(), 2);
    
    let terminals_on_router20: Vec<_> = terminals.iter()
        .filter(|t| t.connected_router_id == Some(20))
        .collect();
    assert_eq!(terminals_on_router20.len(), 1);
    
    let terminals_on_router30: Vec<_> = terminals.iter()
        .filter(|t| t.connected_router_id == Some(30))
        .collect();
    assert_eq!(terminals_on_router30.len(), 0);
}

#[test]
fn test_send_ping_from_terminal() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    manager.connect_terminal_to_router(id, 10, 1).unwrap();
    
    let result = manager.send_ping_from_terminal(id, "10.0.0.1".to_string(), 0.0);
    assert!(result.is_ok());
    
    // Test send from non-existent terminal
    let result2 = manager.send_ping_from_terminal(999, "10.0.0.1".to_string(), 0.0);
    assert!(result2.is_err());
}

#[test]
fn test_process_all_packet_queues() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    manager.connect_terminal_to_router(id, 10, 1).unwrap();
    
    // Send ping which will create packets in the queue
    manager.send_ping_from_terminal(id, "10.0.0.1".to_string(), 0.0).unwrap();
    
    let packets = manager.process_all_packet_queues(1.0);
    assert!(!packets.is_empty());
}

#[test]
fn test_process_icmp_packet() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    // Test echo request
    let request = create_test_packet("192.168.1.50", "192.168.1.100");
    
    let reply = manager.process_icmp_packet(id, request, 0.0);
    assert!(reply.is_ok());
    assert!(reply.unwrap().is_some());
    
    // Test echo reply
    let echo_reply = ICMPPacket {
        packet_type: ICMPType::EchoReply,
        code: 0,
        checksum: 0,
        identifier: 1234,
        sequence_number: 1,
        data: vec![0; 56],
        source_ip: "10.0.0.1".to_string(),
        destination_ip: "192.168.1.100".to_string(),
        ttl: 63,
        original_packet: None,
    };
    
    let result = manager.process_icmp_packet(id, echo_reply, 0.0);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // Echo replies don't generate responses
}

#[test]
fn test_update_statistics() {
    let mut manager = create_test_manager();
    
    let id1 = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    let id2 = manager.add_terminal(
        "Terminal-2".to_string(),
        "192.168.1.101".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    // Set one terminal as failed
    manager.set_terminal_failed(id1, true).unwrap();
    
    // Queue some packets
    // Send pings from terminals
    manager.send_ping_from_terminal(id1, "10.0.0.1".to_string(), 0.0).unwrap();
    manager.send_ping_from_terminal(id2, "10.0.0.2".to_string(), 0.0).unwrap();
    
    manager.update_statistics(100.0);
    
    assert_eq!(manager.get_statistics().total_terminals, 2);
    assert_eq!(manager.get_statistics().active_terminals, 1);
    assert_eq!(manager.get_statistics().failed_terminals, 1);
    assert_eq!(manager.get_statistics().last_update_time, 100.0);
}

#[test]
fn test_reset_statistics() {
    let mut manager = create_test_manager();
    
    // Add terminals and generate some statistics
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    // Send ping and process packets
    manager.send_ping_from_terminal(id, "10.0.0.1".to_string(), 0.0).unwrap();
    manager.process_all_packet_queues(1.0);
    
    // Reset statistics
    manager.reset_statistics();
    
    assert_eq!(manager.get_statistics().total_packets_sent, 0);
    assert_eq!(manager.get_statistics().total_packets_received, 0);
    assert_eq!(manager.get_statistics().total_packets_dropped, 0);
    // Terminal count should remain
    assert_eq!(manager.get_statistics().total_terminals, 1);
}

#[test]
fn test_clear_all_terminals() {
    let mut manager = create_test_manager();
    
    manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    manager.add_terminal(
        "Terminal-2".to_string(),
        "192.168.1.101".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    assert_eq!(manager.get_statistics().total_terminals, 2);
    
    // Remove terminals one by one since clear_all_terminals doesn't exist
    let terminals = manager.get_all_terminals_info();
    for terminal_info in terminals {
        manager.remove_terminal(terminal_info.id).unwrap();
    }
    
    assert_eq!(manager.get_statistics().total_terminals, 0);
}

#[test]
fn test_get_terminal_info() {
    let mut manager = create_test_manager();
    
    let id = manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    manager.connect_terminal_to_router(id, 10, 1).unwrap();
    
    let info = manager.get_terminal_info(id);
    assert!(info.is_ok());
    
    let terminal_info = info.unwrap();
    assert_eq!(terminal_info.id, id);
    assert_eq!(terminal_info.name, "Terminal-1");
    assert_eq!(terminal_info.ip_address, "192.168.1.100");
    assert_eq!(terminal_info.connected_router_id, Some(10));
    assert!(!terminal_info.is_failed);
    assert_eq!(terminal_info.connected_router_id, Some(10));
}

#[test]
fn test_config_management() {
    let mut manager = create_test_manager();
    
    // Test default config
    assert_eq!(manager.get_config().max_terminals, 1000);
    assert_eq!(manager.get_config().packet_delivery_delay, 0.001);
    
    // Modify config
    let new_config = ManagerConfig {
        max_terminals: 500,
        packet_delivery_delay: 0.002,
        ..Default::default()
    };
    manager.update_config(new_config);
    
    assert_eq!(manager.get_config().max_terminals, 500);
    assert_eq!(manager.get_config().packet_delivery_delay, 0.002);
}

#[test]
fn test_serialization() {
    let mut manager = create_test_manager();
    
    manager.add_terminal(
        "Terminal-1".to_string(),
        "192.168.1.100".to_string(),
        "255.255.255.0".to_string(),
        "192.168.1.1".to_string(),
    ).unwrap();
    
    // Serialize
    let json = serde_json::to_string(&manager).unwrap();
    
    // Deserialize
    let manager2: TerminalManager = serde_json::from_str(&json).unwrap();
    
    assert_eq!(manager2.get_statistics().total_terminals, 1);
    assert_eq!(manager2.get_all_terminals_info()[0].name, "Terminal-1");
}