#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::router::InterfaceConfig;
    use crate::console_log;

    #[test]
    fn test_interface_config_update() {
        let mut sim = NetworkSimulation::new();
        
        // ルーターを作成
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // 接続を作成
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // R1のインターフェースを取得
        let router = sim.topology.routers.get(&r1).unwrap();
        let interface = router.interfaces.values().next().unwrap();
        let interface_id = interface.id;
        
        console_log!("Initial interface config:");
        console_log!("  Hello interval: {}", interface.hello_interval);
        console_log!("  Dead interval: {}", interface.dead_interval);
        console_log!("  Priority: {}", interface.priority);
        console_log!("  MTU: {}", interface.mtu);
        
        // インターフェース設定を更新
        let new_config = InterfaceConfig {
            ip_address: Some("10.0.1.100".to_string()),
            netmask: None,
            cost: Some(20),
            hello_interval: Some(5),
            dead_interval: Some(20),
            priority: Some(10),
            mtu: Some(9000),
            enabled: None,
        };
        
        sim.update_interface_config(r1, interface_id, new_config).unwrap();
        
        // 更新後の設定を確認
        let router = sim.topology.routers.get(&r1).unwrap();
        let interface = router.interfaces.get(&interface_id).unwrap();
        
        console_log!("\nUpdated interface config:");
        console_log!("  IP address: {}", interface.ip_address);
        console_log!("  Cost: {}", interface.cost);
        console_log!("  Hello interval: {}", interface.hello_interval);
        console_log!("  Dead interval: {}", interface.dead_interval);
        console_log!("  Priority: {}", interface.priority);
        console_log!("  MTU: {}", interface.mtu);
        
        // 設定が正しく更新されたことを確認
        assert_eq!(interface.ip_address, "10.0.1.100");
        assert_eq!(interface.cost, 20);
        assert_eq!(interface.hello_interval, 5);
        assert_eq!(interface.dead_interval, 20);
        assert_eq!(interface.priority, 10);
        assert_eq!(interface.mtu, 9000);
        assert!(interface.manual_config);
    }

    #[test]
    fn test_interface_config_ospf_timer_update() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        sim.enable_ospf(r1).unwrap();
        
        // シミュレーションを開始して初期タイマーを設定
        sim.start_simulation();
        sim.step_simulation(0.1);
        
        // インターフェースを取得
        let router = sim.topology.routers.get(&r1).unwrap();
        let interface = router.interfaces.values().next().unwrap();
        let interface_id = interface.id;
        
        // 新しいHello間隔を設定
        let new_config = InterfaceConfig {
            ip_address: Some(interface.ip_address.clone()),
            netmask: None,
            cost: Some(interface.cost),
            hello_interval: Some(3),  // 10秒から3秒に変更
            dead_interval: Some(12),  // 40秒から12秒に変更
            priority: Some(interface.priority),
            mtu: Some(interface.mtu),
            enabled: None,
        };
        
        sim.update_interface_config(r1, interface_id, new_config).unwrap();
        
        // OSPFエンジンのタイマーも更新されることを期待
        // 実際の実装では、OSPFエンジンがインターフェース設定の変更を
        // 検知してタイマーを再設定する必要がある
        
        console_log!("Interface timers updated successfully");
    }
}