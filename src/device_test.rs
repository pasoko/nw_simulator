#[cfg(test)]
mod tests {
    use crate::device::*;

    #[test]
    fn test_host_device_creation() {
        let host = HostDevice::new(
            1000,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        assert_eq!(host.id, 1000);
        assert_eq!(host.name, "Host1");
        assert_eq!(host.ip_address, "192.168.1.10");
        assert_eq!(host.netmask, "255.255.255.0");
        assert_eq!(host.default_gateway, "192.168.1.1");
        assert!(!host.is_failed);
        assert!(host.connected_router_id.is_none());
    }

    #[test]
    fn test_host_router_connection() {
        let mut host = HostDevice::new(
            1000,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        host.connect_to_router(1, 5);
        assert_eq!(host.connected_router_id, Some(1));
        assert_eq!(host.connected_interface_id, Some(5));

        host.disconnect();
        assert!(host.connected_router_id.is_none());
        assert!(host.connected_interface_id.is_none());
    }

    #[test]
    fn test_same_subnet_check() {
        let host = HostDevice::new(
            1000,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        // 同一サブネット
        assert!(host.is_same_subnet("192.168.1.20"));
        assert!(host.is_same_subnet("192.168.1.254"));

        // 異なるサブネット
        assert!(!host.is_same_subnet("192.168.2.10"));
        assert!(!host.is_same_subnet("10.0.0.1"));

        // 無効なIPアドレス
        assert!(!host.is_same_subnet("invalid"));
        assert!(!host.is_same_subnet(""));
    }

    #[test]
    fn test_next_hop_determination() {
        let host = HostDevice::new(
            1000,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        // 同一サブネット -> 直接送信
        assert_eq!(host.get_next_hop("192.168.1.20"), "192.168.1.20");

        // 異なるサブネット -> デフォルトゲートウェイ経由
        assert_eq!(host.get_next_hop("8.8.8.8"), "192.168.1.1");
        assert_eq!(host.get_next_hop("10.0.0.1"), "192.168.1.1");
    }

    #[test]
    fn test_arp_table() {
        let mut host = HostDevice::new(
            1000,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        assert!(host.arp_table.is_empty());

        host.add_arp_entry("192.168.1.1".to_string(), "aa:bb:cc:dd:ee:ff".to_string());
        host.add_arp_entry("192.168.1.20".to_string(), "11:22:33:44:55:66".to_string());

        assert_eq!(host.arp_table.len(), 2);
        assert_eq!(host.arp_table.get("192.168.1.1"), Some(&"aa:bb:cc:dd:ee:ff".to_string()));
        assert_eq!(host.arp_table.get("192.168.1.20"), Some(&"11:22:33:44:55:66".to_string()));
    }

    #[test]
    fn test_icmp_packet_creation() {
        let echo_request = ICMPPacket::new_echo_request(1234, 1);
        assert_eq!(echo_request.packet_type, ICMPType::EchoRequest);
        assert_eq!(echo_request.code, 0);
        assert_eq!(echo_request.identifier, 1234);
        assert_eq!(echo_request.sequence_number, 1);
        assert_eq!(echo_request.data.len(), 32);

        let echo_reply = ICMPPacket::new_echo_reply(1234, 1);
        assert_eq!(echo_reply.packet_type, ICMPType::EchoReply);
        assert_eq!(echo_reply.code, 0);
        assert_eq!(echo_reply.identifier, 1234);
        assert_eq!(echo_reply.sequence_number, 1);
        assert_eq!(echo_reply.data.len(), 32);
    }

    #[test]
    fn test_device_type() {
        assert_eq!(DeviceType::Router as u8, 0);
        assert_eq!(DeviceType::Host as u8, 1);
    }
}