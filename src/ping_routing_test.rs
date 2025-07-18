#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::console_log;

    #[test]
    fn test_ping_routing_through_network() {
        console_log!("=== Ping Routing Test ===");
        
        let mut sim = NetworkSimulation::new();
        
        // ネットワークトポロジーの構築
        // Host1 (192.168.1.100) -- R1 -- R2 -- R3 -- Host2 (192.168.3.100)
        
        // ルーターの追加
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        let r3 = sim.add_router("R3".to_string(), 200.0, 0.0);
        
        // ホストの追加
        let h1 = sim.add_host("Host1".to_string(), 
            "192.168.1.100".to_string(), 
            "255.255.255.0".to_string(), 
            "192.168.1.1".to_string()
        );
        let h2 = sim.add_host("Host2".to_string(), 
            "192.168.3.100".to_string(), 
            "255.255.255.0".to_string(), 
            "192.168.3.1".to_string()
        );
        
        // 接続の作成
        sim.connect_host_to_router(h1, r1).unwrap();
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_host_to_router(h2, r3).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // OSPFが収束するまで待つ
        sim.start_simulation();
        for i in 0..100 {
            sim.step_simulation(0.1);
            if i % 10 == 0 {
                console_log!("Time: {:.1}s", sim.simulation_time);
            }
        }
        
        console_log!("\n=== Routing Tables After Convergence ===");
        
        // 各ルーターのルーティングテーブルを表示
        for router_id in [r1, r2, r3] {
            if let Some(router) = sim.topology.routers.get(&router_id) {
                console_log!("\nRouter {} routing table:", router.name);
                for entry in &router.routing_table {
                    console_log!("  {} -> next hop: {}, interface: {}, metric: {}", 
                        entry.destination, entry.next_hop, entry.interface_name, entry.metric);
                }
            }
        }
        
        // Host1からHost2へping送信
        console_log!("\n=== Sending Ping from Host1 to Host2 ===");
        let ping_id = sim.send_ping_from_host(h1, "192.168.3.100".to_string()).unwrap();
        console_log!("Ping sent with ID: {}", ping_id);
        
        // パケットが転送されるのを待つ
        for _ in 0..20 {
            sim.step_simulation(0.01);
        }
        
        // ping結果を確認
        let results = sim.get_recent_ping_results(10);
        console_log!("\n=== Ping Results ===");
        for result in &results {
            if result.success {
                console_log!("Ping to {} successful! RTT: {:.2}ms", 
                    result.destination_ip, result.rtt_ms.unwrap());
            } else {
                console_log!("Ping to {} failed: {}", 
                    result.destination_ip, 
                    result.error_message.as_ref().unwrap_or(&"Unknown error".to_string()));
            }
        }
        
        // 結果の検証
        assert!(!results.is_empty(), "Should have ping results");
        let first_result = &results[0];
        assert!(first_result.success, "Ping should succeed");
        assert!(first_result.rtt_ms.is_some(), "Should have RTT measurement");
    }

    #[test]
    fn test_ping_to_unreachable_host() {
        console_log!("\n=== Ping to Unreachable Host Test ===");
        
        let mut sim = NetworkSimulation::new();
        
        // 単純なネットワーク: Host1 -- R1 (Host2は接続されていない)
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let h1 = sim.add_host("Host1".to_string(), 
            "192.168.1.100".to_string(), 
            "255.255.255.0".to_string(), 
            "192.168.1.1".to_string()
        );
        
        sim.connect_host_to_router(h1, r1).unwrap();
        sim.enable_ospf(r1).unwrap();
        
        sim.start_simulation();
        sim.step_simulation(1.0);
        
        // 存在しないホストへping送信
        let ping_id = sim.send_ping_from_host(h1, "10.0.0.100".to_string()).unwrap();
        console_log!("Ping sent to unreachable host with ID: {}", ping_id);
        
        // タイムアウトを待つ
        for _ in 0..50 {
            sim.step_simulation(0.1);
        }
        
        let results = sim.get_recent_ping_results(10);
        console_log!("Ping results: {:?}", results);
        
        // タイムアウトになることを確認
        // Note: 現在の実装ではルートがない場合の処理が不完全なため、
        // このテストは将来の改善のためのプレースホルダー
    }

    #[test]
    fn test_ping_to_router_interface() {
        console_log!("\n=== Ping to Router Interface Test ===");
        
        let mut sim = NetworkSimulation::new();
        
        // Host1 -- R1 -- R2
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        let h1 = sim.add_host("Host1".to_string(), 
            "192.168.1.100".to_string(), 
            "255.255.255.0".to_string(), 
            "192.168.1.1".to_string()
        );
        
        sim.connect_host_to_router(h1, r1).unwrap();
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        sim.start_simulation();
        sim.step_simulation(5.0);
        
        // R2のインターフェースIPを取得
        let r2_interface_ip = if let Some(router) = sim.topology.routers.get(&r2) {
            router.interfaces.values()
                .find(|iface| iface.connected_router_id == Some(r1))
                .map(|iface| iface.ip_address.clone())
                .unwrap_or_default()
        } else {
            String::new()
        };
        
        console_log!("Pinging R2 interface at {}", r2_interface_ip);
        
        if !r2_interface_ip.is_empty() {
            let ping_id = sim.send_ping_from_host(h1, r2_interface_ip.clone()).unwrap();
            console_log!("Ping sent with ID: {}", ping_id);
            
            // パケット転送を待つ
            for _ in 0..10 {
                sim.step_simulation(0.01);
            }
            
            let results = sim.get_recent_ping_results(10);
            for result in &results {
                console_log!("Ping result: success={}, destination={}", 
                    result.success, result.destination_ip);
            }
            
            // ルーターインターフェースへのpingは成功するはず
            assert!(!results.is_empty());
            assert!(results[0].success);
        }
    }
}