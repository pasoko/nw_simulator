#[cfg(test)]
mod interface_naming_tests {
    use crate::network::NetworkTopology;
    use crate::router::RouterState;

    #[test]
    fn test_interface_naming_single_router() {
        let mut topology = NetworkTopology::new();
        
        // ルーター作成
        let router_id = topology.add_router("R1".to_string());
        
        // 別のルーターと接続してインターフェースを作成
        let router2_id = topology.add_router("R2".to_string());
        let router3_id = topology.add_router("R3".to_string());
        
        // R1と他のルーターを接続
        topology.connect_routers(router_id, router2_id, 10).unwrap();
        topology.connect_routers(router_id, router3_id, 20).unwrap();
        
        // R1のインターフェース名を確認
        let router = topology.routers.get(&router_id).unwrap();
        let interface_names: Vec<String> = router.interfaces.values()
            .map(|iface| iface.name.clone())
            .collect();
        
        assert_eq!(interface_names.len(), 2);
        assert!(interface_names.contains(&"IFR1-1".to_string()));
        assert!(interface_names.contains(&"IFR1-2".to_string()));
    }

    #[test]
    fn test_interface_naming_multiple_routers() {
        let mut topology = NetworkTopology::new();
        
        // 3つのルーターを作成
        let r1_id = topology.add_router("Router1".to_string());
        let r2_id = topology.add_router("Router2".to_string());
        let r3_id = topology.add_router("Router3".to_string());
        
        // 接続を作成
        topology.connect_routers(r1_id, r2_id, 10).unwrap();
        topology.connect_routers(r2_id, r3_id, 15).unwrap();
        topology.connect_routers(r1_id, r3_id, 20).unwrap();
        
        // Router1のインターフェース確認
        let router1 = topology.routers.get(&r1_id).unwrap();
        let r1_interface_names: Vec<String> = router1.interfaces.values()
            .map(|iface| iface.name.clone())
            .collect();
        assert_eq!(r1_interface_names.len(), 2);
        assert!(r1_interface_names.contains(&"IFRouter1-1".to_string()));
        assert!(r1_interface_names.contains(&"IFRouter1-2".to_string()));
        
        // Router2のインターフェース確認
        let router2 = topology.routers.get(&r2_id).unwrap();
        let r2_interface_names: Vec<String> = router2.interfaces.values()
            .map(|iface| iface.name.clone())
            .collect();
        assert_eq!(r2_interface_names.len(), 2);
        assert!(r2_interface_names.contains(&"IFRouter2-1".to_string()));
        assert!(r2_interface_names.contains(&"IFRouter2-2".to_string()));
        
        // Router3のインターフェース確認
        let router3 = topology.routers.get(&r3_id).unwrap();
        let r3_interface_names: Vec<String> = router3.interfaces.values()
            .map(|iface| iface.name.clone())
            .collect();
        assert_eq!(r3_interface_names.len(), 2);
        assert!(r3_interface_names.contains(&"IFRouter3-1".to_string()));
        assert!(r3_interface_names.contains(&"IFRouter3-2".to_string()));
    }

    #[test]
    fn test_interface_numbering_sequence() {
        let mut topology = NetworkTopology::new();
        
        // ハブルーターと複数のスポークルーター
        let hub_id = topology.add_router("HUB".to_string());
        let spoke1_id = topology.add_router("SPOKE1".to_string());
        let spoke2_id = topology.add_router("SPOKE2".to_string());
        let spoke3_id = topology.add_router("SPOKE3".to_string());
        let spoke4_id = topology.add_router("SPOKE4".to_string());
        
        // ハブと各スポークを接続
        topology.connect_routers(hub_id, spoke1_id, 10).unwrap();
        topology.connect_routers(hub_id, spoke2_id, 10).unwrap();
        topology.connect_routers(hub_id, spoke3_id, 10).unwrap();
        topology.connect_routers(hub_id, spoke4_id, 10).unwrap();
        
        // HUBルーターのインターフェース名を確認
        let hub = topology.routers.get(&hub_id).unwrap();
        let hub_interface_names: Vec<String> = hub.interfaces.values()
            .map(|iface| iface.name.clone())
            .collect();
        
        assert_eq!(hub_interface_names.len(), 4);
        assert!(hub_interface_names.contains(&"IFHUB-1".to_string()));
        assert!(hub_interface_names.contains(&"IFHUB-2".to_string()));
        assert!(hub_interface_names.contains(&"IFHUB-3".to_string()));
        assert!(hub_interface_names.contains(&"IFHUB-4".to_string()));
    }

    #[test]
    fn test_router_state_interface_counter() {
        let mut router = RouterState::new(1, "TestRouter".to_string());
        
        // 初期状態の確認
        assert_eq!(router.next_interface_number, 1);
        
        // インターフェースを追加
        router.add_interface(crate::router::RouterInterface {
            id: 100,
            name: String::new(),
            ip_address: "10.0.1.1".to_string(),
            netmask: "255.255.255.0".to_string(),
            connected_router_id: Some(2),
            cost: 10,
            enabled: true,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            mtu: 1500,
            manual_config: false,
            auth_config: crate::ospf_auth::AuthConfig::default(),
        });
        
        // カウンターが増加していることを確認
        assert_eq!(router.next_interface_number, 2);
        
        // インターフェース名が正しく設定されていることを確認
        let interface = router.interfaces.get(&100).unwrap();
        assert_eq!(interface.name, "IFTestRouter-1");
        
        // もう一つインターフェースを追加
        router.add_interface(crate::router::RouterInterface {
            id: 101,
            name: String::new(),
            ip_address: "10.0.2.1".to_string(),
            netmask: "255.255.255.0".to_string(),
            connected_router_id: Some(3),
            cost: 20,
            enabled: true,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            mtu: 1500,
            manual_config: false,
            auth_config: crate::ospf_auth::AuthConfig::default(),
        });
        
        assert_eq!(router.next_interface_number, 3);
        let interface2 = router.interfaces.get(&101).unwrap();
        assert_eq!(interface2.name, "IFTestRouter-2");
    }

    #[test]
    fn test_preserve_existing_interface_name() {
        let mut router = RouterState::new(1, "R1".to_string());
        
        // 明示的に名前を設定したインターフェース
        router.add_interface(crate::router::RouterInterface {
            id: 200,
            name: "CustomName".to_string(),
            ip_address: "10.0.1.1".to_string(),
            netmask: "255.255.255.0".to_string(),
            connected_router_id: Some(2),
            cost: 10,
            enabled: true,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            mtu: 1500,
            manual_config: false,
            auth_config: crate::ospf_auth::AuthConfig::default(),
        });
        
        // カスタム名が保持されていることを確認
        let interface = router.interfaces.get(&200).unwrap();
        assert_eq!(interface.name, "CustomName");
        
        // カウンターは変化しない
        assert_eq!(router.next_interface_number, 1);
    }
}