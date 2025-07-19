#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::enhanced_ping::{EnhancedPingManager, PingSessionConfig};
    use crate::device::ICMPType;
    
    #[test]
    fn test_enhanced_ping_integration() {
        let mut sim = NetworkSimulation::new();
        
        // ネットワークトポロジーを作成: Terminal1 -- R1 -- R2 -- R3 -- Terminal2
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 300.0, 100.0);
        
        let terminal1 = sim.add_terminal(
            "Terminal1".to_string(),
            "192.168.1.100".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        let terminal2 = sim.add_terminal(
            "Terminal2".to_string(),
            "192.168.3.100".to_string(),
            "255.255.255.0".to_string(),
            "192.168.3.1".to_string(),
        ).unwrap();
        
        // 接続を設定
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_terminal_to_router(terminal1, r1).unwrap();
        sim.connect_terminal_to_router(terminal2, r3).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // シミュレーションを進めてOSPFコンバージェンスを待つ
        sim.start_simulation();
        for _ in 0..100 {
            sim.step_simulation(0.1);
        }
        
        // 拡張ping機能のテスト
        let mut ping_manager = EnhancedPingManager::new();
        
        let config = PingSessionConfig {
            count: 5,
            interval_seconds: 0.5,
            timeout_seconds: 2.0,
            initial_ttl: 64,
            packet_size: 64,
            ..Default::default()
        };
        
        let session_id = ping_manager.start_ping_session(
            terminal1,
            "192.168.1.100".to_string(),
            "192.168.3.100".to_string(),
            config,
            sim.simulation_time,
        ).unwrap();
        
        // 最初のpingを生成
        let packet = ping_manager.generate_next_ping(session_id, sim.simulation_time)
            .unwrap()
            .unwrap();
        
        assert_eq!(packet.identifier, 1000);
        assert_eq!(packet.sequence_number, 1);
        assert_eq!(packet.ttl, 64);
        assert_eq!(packet.data.len(), 56);  // 64 - 8 (ICMPヘッダー)
        
        // セッション情報を確認
        let session = ping_manager.get_session_info(session_id).unwrap();
        assert_eq!(session.packets_sent, 1);
        assert_eq!(session.config.count, 5);
    }
    
    #[test]
    fn test_ping_with_ttl_expiry() {
        let mut ping_manager = EnhancedPingManager::new();
        
        let config = PingSessionConfig {
            initial_ttl: 2,  // 低いTTLを設定
            ..Default::default()
        };
        
        let session_id = ping_manager.start_ping_session(
            1,
            "10.0.0.1".to_string(),
            "10.0.0.100".to_string(),
            config,
            0.0,
        ).unwrap();
        
        let packet = ping_manager.generate_next_ping(session_id, 0.0)
            .unwrap()
            .unwrap();
        
        assert_eq!(packet.ttl, 2);
        
        // TTL期限切れエラーを処理
        let result = ping_manager.process_icmp_error(
            ICMPType::TimeExceeded,
            packet.identifier,
            packet.sequence_number,
            0.1,
        ).unwrap();
        
        assert!(!result.success);
        assert!(result.error_message.is_some());
        assert!(result.error_message.unwrap().contains("TTL expired"));
    }
    
    #[test]
    fn test_continuous_ping_session() {
        let mut ping_manager = EnhancedPingManager::new();
        
        let config = PingSessionConfig {
            count: 3,
            interval_seconds: 0.1,
            ..Default::default()
        };
        
        let session_id = ping_manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "192.168.2.20".to_string(),
            config,
            0.0,
        ).unwrap();
        
        let mut current_time = 0.0;
        let mut sent_packets = Vec::new();
        
        // 連続してpingを送信
        for i in 0..3 {
            if let Some(packet) = ping_manager.generate_next_ping(session_id, current_time).unwrap() {
                sent_packets.push(packet.clone());
                
                // シミュレート：50ms後にreply受信
                current_time += 0.05;
                let result = ping_manager.process_echo_reply(
                    packet.identifier,
                    packet.sequence_number,
                    56,
                    current_time,
                ).unwrap();
                
                assert!(result.success);
                assert!((result.rtt_ms.unwrap() - 50.0).abs() < 0.001);
                assert_eq!(result.hop_count, Some(8));  // 64 - 56
            }
            
            current_time = (i + 1) as f64 * 0.1;
        }
        
        assert_eq!(sent_packets.len(), 3);
        
        // セッションサマリーを取得
        let summary = ping_manager.stop_session(session_id).unwrap();
        assert_eq!(summary.packets_sent, 3);
        assert_eq!(summary.packets_received, 3);
        assert_eq!(summary.packets_lost, 0);
        assert_eq!(summary.loss_percentage, 0.0);
        assert!((summary.min_rtt_ms.unwrap() - 50.0).abs() < 0.001);
        assert!((summary.max_rtt_ms.unwrap() - 50.0).abs() < 0.001);
        assert!((summary.avg_rtt_ms.unwrap() - 50.0).abs() < 0.001);
    }
    
    #[test]
    fn test_ping_statistics() {
        let mut ping_manager = EnhancedPingManager::new();
        
        // 複数のセッションを作成
        let session1 = ping_manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "8.8.8.8".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        let session2 = ping_manager.start_ping_session(
            2,
            "192.168.1.20".to_string(),
            "1.1.1.1".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        // 各セッションでpingを送信
        for session_id in [session1, session2] {
            let packet = ping_manager.generate_next_ping(session_id, 0.0).unwrap().unwrap();
            
            // session1は成功、session2は失敗
            if session_id == session1 {
                let _ = ping_manager.process_echo_reply(
                    packet.identifier,
                    packet.sequence_number,
                    60,
                    0.1,
                ).unwrap();
            }
        }
        
        // タイムアウトチェック
        ping_manager.check_timeouts(4.0);
        
        // グローバル統計を確認
        let stats = ping_manager.get_global_statistics();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.total_packets_sent, 2);
        assert_eq!(stats.total_packets_received, 1);
        assert_eq!(stats.total_packets_lost, 1);
    }
    
    #[test]
    fn test_ping_with_variable_rtt() {
        let mut ping_manager = EnhancedPingManager::new();
        
        let session_id = ping_manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "192.168.2.20".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        // 異なるRTTで複数のpingを送信
        let rtts = vec![20.0, 50.0, 30.0, 100.0, 10.0];
        
        for (i, expected_rtt) in rtts.iter().enumerate() {
            let packet = ping_manager.generate_next_ping(session_id, i as f64).unwrap().unwrap();
            
            let result = ping_manager.process_echo_reply(
                packet.identifier,
                packet.sequence_number,
                56,
                i as f64 + expected_rtt / 1000.0,
            ).unwrap();
            
            assert!((result.rtt_ms.unwrap() - *expected_rtt).abs() < 0.001);
        }
        
        let session = ping_manager.get_session_info(session_id).unwrap();
        assert!((session.min_rtt_ms.unwrap() - 10.0).abs() < 0.001);
        assert!((session.max_rtt_ms.unwrap() - 100.0).abs() < 0.001);
        assert!((session.avg_rtt_ms.unwrap() - 42.0).abs() < 0.001);  // (20+50+30+100+10)/5
    }
    
    #[test]
    fn test_destination_unreachable() {
        let mut ping_manager = EnhancedPingManager::new();
        
        let session_id = ping_manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "192.168.99.99".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        let packet = ping_manager.generate_next_ping(session_id, 0.0).unwrap().unwrap();
        
        // Destination Unreachableエラーを処理
        let result = ping_manager.process_icmp_error(
            ICMPType::DestinationUnreachable,
            packet.identifier,
            packet.sequence_number,
            0.1,
        ).unwrap();
        
        assert!(!result.success);
        assert_eq!(result.error_message, Some("Destination unreachable".to_string()));
        
        let session = ping_manager.get_session_info(session_id).unwrap();
        assert_eq!(session.packets_lost, 1);
    }
    
    #[test]
    fn test_session_completion() {
        let mut ping_manager = EnhancedPingManager::new();
        
        let config = PingSessionConfig {
            count: 2,
            ..Default::default()
        };
        
        let session_id = ping_manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "192.168.2.20".to_string(),
            config,
            0.0,
        ).unwrap();
        
        // 2回pingを送信
        for i in 0..2 {
            let _ = ping_manager.generate_next_ping(session_id, i as f64).unwrap();
        }
        
        assert!(ping_manager.is_session_complete(session_id));
        
        // 3回目は送信されない
        let result = ping_manager.generate_next_ping(session_id, 2.0).unwrap();
        assert!(result.is_none());
    }
}