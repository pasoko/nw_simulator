#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::terminal_device::{TerminalDevice, TerminalConfig};
    use crate::terminal_manager::{TerminalManager, ManagerConfig};
    use crate::device::{ICMPPacket, ICMPType};
    
    #[test]
    fn test_independent_terminal_creation() {
        let mut sim = NetworkSimulation::new();
        
        // シミュレーションに端末マネージャーが統合されていることをテスト
        let terminal_id = sim.add_terminal(
            "Terminal1".to_string(),
            "192.168.1.100".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        assert!(terminal_id.is_ok());
        let id = terminal_id.unwrap();
        
        // 端末情報を取得
        let terminal_info = sim.get_terminal_info(id);
        assert!(terminal_info.is_ok());
        
        let info = terminal_info.unwrap();
        assert_eq!(info.name, "Terminal1");
        assert_eq!(info.ip_address, "192.168.1.100");
        assert_eq!(info.default_gateway, "192.168.1.1");
        assert!(!info.is_failed);
    }
    
    #[test]
    fn test_terminal_router_connection() {
        let mut sim = NetworkSimulation::new();
        
        // ルーターとターミナルを作成
        let router_id = sim.add_router("Router1".to_string(), 100.0, 100.0);
        let terminal_id = sim.add_terminal(
            "Terminal1".to_string(),
            "192.168.1.100".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(router_id).unwrap();
        
        // 端末をルーターに接続
        let result = sim.connect_terminal_to_router(terminal_id, router_id);
        assert!(result.is_ok());
        
        // 接続状態を確認
        let terminal_info = sim.get_terminal_info(terminal_id).unwrap();
        assert_eq!(terminal_info.connected_router_id, Some(router_id));
    }
    
    #[test]
    fn test_independent_ping_functionality() {
        let mut sim = NetworkSimulation::new();
        
        // ネットワークトポロジーを作成: Terminal1 -- Router1 -- Router2 -- Terminal2
        let router1_id = sim.add_router("Router1".to_string(), 100.0, 100.0);
        let router2_id = sim.add_router("Router2".to_string(), 200.0, 100.0);
        
        let terminal1_id = sim.add_terminal(
            "Terminal1".to_string(),
            "192.168.1.100".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        let terminal2_id = sim.add_terminal(
            "Terminal2".to_string(),
            "192.168.2.100".to_string(),
            "255.255.255.0".to_string(),
            "192.168.2.1".to_string(),
        ).unwrap();
        
        // 接続を設定
        sim.connect_routers(router1_id, router2_id, 10).unwrap();
        sim.connect_terminal_to_router(terminal1_id, router1_id).unwrap();
        sim.connect_terminal_to_router(terminal2_id, router2_id).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(router1_id).unwrap();
        sim.enable_ospf(router2_id).unwrap();
        
        // 端末1から端末2にpingを送信
        let ping_result = sim.send_ping_from_terminal(
            terminal1_id,
            "192.168.2.100".to_string(),
        );
        
        assert!(ping_result.is_ok());
        
        // 統計情報を確認
        let terminal1_info = sim.get_terminal_info(terminal1_id).unwrap();
        assert_eq!(terminal1_info.statistics.icmp_echo_requests_sent, 1);
    }
    
    #[test]
    fn test_terminal_device_independence() {
        let mut terminal = TerminalDevice::new(
            1,
            "IndependentTerminal".to_string(),
            "10.0.1.100".to_string(),
            "255.255.255.0".to_string(),
            "10.0.1.1".to_string(),
        );
        
        // ルーターなしでの独立動作をテスト
        assert_eq!(terminal.resolve_next_hop("10.0.1.50"), Some("10.0.1.50".to_string()));
        assert_eq!(terminal.resolve_next_hop("8.8.8.8"), Some("10.0.1.1".to_string()));
        
        // ARPテーブルの管理
        terminal.add_arp_entry("10.0.1.1", "aa:bb:cc:dd:ee:ff");
        assert!(terminal.lookup_arp("10.0.1.1").is_some());
        
        // ルートエントリの追加
        terminal.add_route(
            "172.16.0.0",
            "255.255.0.0",
            "10.0.1.2",
            5
        );
        
        assert_eq!(terminal.routing_table.len(), 2); // デフォルトルート + 追加ルート
    }
    
    #[test]
    fn test_terminal_packet_queue_management() {
        let mut terminal = TerminalDevice::new(
            1,
            "QueueTestTerminal".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        // ルーターに接続
        terminal.connect_to_router(100, 1);
        
        // 複数のpingを送信
        for i in 1..=5 {
            let result = terminal.start_ping(format!("8.8.8.{}", i), 64, 1000 + i, 0.0);
            assert!(result.is_ok());
        }
        
        assert_eq!(terminal.packet_queue.len(), 5);
        assert_eq!(terminal.statistics.icmp_echo_requests_sent, 5);
        
        // パケットキューを処理
        let packets = terminal.process_packet_queue(1.0);
        assert_eq!(packets.len(), 5); // 最初の送信
        assert_eq!(terminal.packet_queue.len(), 0); // キューは空になっているはず
    }
    
    #[test]
    fn test_terminal_failure_handling() {
        let mut terminal = TerminalDevice::new(
            1,
            "FailureTestTerminal".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        terminal.connect_to_router(100, 1);
        
        // 正常時のping送信
        let result = terminal.start_ping("8.8.8.8".to_string(), 64, 1000, 0.0);
        assert!(result.is_ok());
        assert_eq!(terminal.packet_queue.len(), 1);
        
        // 障害を設定
        terminal.set_failed(true);
        assert!(terminal.is_failed);
        assert_eq!(terminal.packet_queue.len(), 0); // キューがクリアされる
        
        // 障害時のping送信（失敗するはず）
        let result = terminal.start_ping("8.8.8.8".to_string(), 64, 1001, 1.0);
        assert!(result.is_err());
        
        // 復旧
        terminal.set_failed(false);
        assert!(!terminal.is_failed);
        
        // 復旧後のping送信
        let result = terminal.start_ping("8.8.8.8".to_string(), 64, 1002, 2.0);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_terminal_manager_functionality() {
        let mut manager = TerminalManager::new();
        
        // 複数の端末を追加
        let terminal1_id = manager.add_terminal(
            "Terminal1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        let terminal2_id = manager.add_terminal(
            "Terminal2".to_string(),
            "192.168.1.20".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        assert_eq!(manager.get_all_terminals_info().len(), 2);
        
        // IPアドレスによる検索
        assert_eq!(manager.find_terminal_by_ip("192.168.1.10"), Some(terminal1_id));
        assert_eq!(manager.find_terminal_by_ip("192.168.1.20"), Some(terminal2_id));
        assert_eq!(manager.find_terminal_by_ip("192.168.1.30"), None);
        
        // 端末をルーターに接続
        manager.connect_terminal_to_router(terminal1_id, 100, 1).unwrap();
        manager.connect_terminal_to_router(terminal2_id, 100, 2).unwrap();
        
        // ping送信
        let result1 = manager.send_ping_from_terminal(terminal1_id, "8.8.8.8".to_string(), 0.0);
        let result2 = manager.send_ping_from_terminal(terminal2_id, "8.8.4.4".to_string(), 0.0);
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // 統計更新
        manager.update_statistics(10.0);
        let stats = manager.get_statistics();
        assert_eq!(stats.total_terminals, 2);
        assert_eq!(stats.active_terminals, 2);
        assert_eq!(stats.failed_terminals, 0);
    }
    
    #[test]
    fn test_icmp_echo_reply_processing() {
        let mut terminal = TerminalDevice::new(
            1,
            "EchoTestTerminal".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        // Echo Requestを受信してReplyを生成
        let echo_request = ICMPPacket::new_echo_request(1000, 1)
            .with_addresses("192.168.1.20".to_string(), "192.168.1.10".to_string());
        
        let echo_reply = terminal.process_echo_request(&echo_request);
        assert!(echo_reply.is_some());
        
        let reply = echo_reply.unwrap();
        assert_eq!(reply.packet_type, ICMPType::EchoReply);
        assert_eq!(reply.identifier, 1000);
        assert_eq!(reply.sequence_number, 1);
        assert_eq!(reply.source_ip, "192.168.1.10");
        assert_eq!(reply.destination_ip, "192.168.1.20");
        assert_eq!(terminal.statistics.icmp_echo_replies_sent, 1);
    }
    
    #[test]
    fn test_custom_terminal_configuration() {
        let mut terminal = TerminalDevice::new(
            1,
            "ConfigTestTerminal".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        // カスタム設定を適用
        let custom_config = TerminalConfig {
            max_retries: 5,
            retry_interval: 0.5,
            packet_timeout: 60.0,
            arp_timeout: 600.0,
            max_queue_size: 200,
            icmp_id_base: 2000,
        };
        
        terminal.update_config(custom_config.clone());
        assert_eq!(terminal.config.max_retries, 5);
        assert_eq!(terminal.config.retry_interval, 0.5);
        assert_eq!(terminal.config.icmp_id_base, 2000);
    }
    
    #[test]
    fn test_manager_with_custom_config() {
        let custom_config = ManagerConfig {
            max_terminals: 500,
            packet_delivery_delay: 0.002,
            statistics_update_interval: 5.0,
            cleanup_interval: 30.0,
        };
        
        let manager = TerminalManager::with_config(custom_config.clone());
        assert_eq!(manager.get_config().max_terminals, 500);
        assert_eq!(manager.get_config().packet_delivery_delay, 0.002);
    }
}