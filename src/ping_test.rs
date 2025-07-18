#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::device::ICMPType;

    #[test]
    fn test_ping_integration() {
        let mut sim = NetworkSimulation::new();

        // ルーターを作成
        let router1 = sim.add_router("Router1".to_string(), 100.0, 100.0);
        let router2 = sim.add_router("Router2".to_string(), 200.0, 100.0);

        // ルーター間を接続
        assert!(sim.connect_routers(router1, router2, 10).is_ok());

        // ホストを作成
        let host1 = sim.add_host(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        let host2 = sim.add_host(
            "Host2".to_string(),
            "192.168.2.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.2.1".to_string(),
        );

        // ホストをルーターに接続
        assert!(sim.connect_host_to_router(host1, router1).is_ok());
        assert!(sim.connect_host_to_router(host2, router2).is_ok());

        // Host1からHost2へping送信
        let result = sim.send_ping_from_host(host1, "192.168.2.10".to_string());
        assert!(result.is_ok());
        let identifier = result.unwrap();
        assert!(identifier > 0);

        // ping結果が存在しないことを確認（まだ応答が返ってきていない）
        let results = sim.get_recent_ping_results(10);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_ping_to_unconnected_host() {
        let mut sim = NetworkSimulation::new();

        // 接続されていないホストを作成
        let host = sim.add_host(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        // ping送信を試みる
        let result = sim.send_ping_from_host(host, "8.8.8.8".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Host is not connected to any router");
    }

    #[test]
    fn test_ping_from_failed_host() {
        let mut sim = NetworkSimulation::new();
        let router = sim.add_router("Router1".to_string(), 100.0, 100.0);

        let host = sim.add_host(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );

        assert!(sim.connect_host_to_router(host, router).is_ok());

        // ホストを障害状態にする
        if let Some(h) = sim.topology.hosts.get_mut(&host) {
            h.is_failed = true;
        }

        // ping送信を試みる
        let result = sim.send_ping_from_host(host, "8.8.8.8".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Host is failed");
    }

    #[test]
    fn test_network_topology_with_hosts() {
        let mut sim = NetworkSimulation::new();

        // ルーターを追加
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);

        // ホストを追加
        let h1 = sim.add_host(
            "H1".to_string(),
            "10.0.1.10".to_string(),
            "255.255.255.0".to_string(),
            "10.0.1.1".to_string(),
        );

        // デバイスタイプの確認
        assert_eq!(sim.topology.get_device_type(r1), Some(crate::device::DeviceType::Router));
        assert_eq!(sim.topology.get_device_type(h1), Some(crate::device::DeviceType::Host));
        assert!(sim.topology.get_device_type(9999).is_none());
    }
}