#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::router::InterfaceConfig;
    use crate::ospf_auth::AuthType;
    use crate::console_log;

    #[test]
    fn test_interface_auth_configuration() {
        let mut sim = NetworkSimulation::new();
        
        // 2つのルーターを作成
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // 接続を作成
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // R1のインターフェースを取得
        let router1 = sim.topology.routers.get(&r1).unwrap();
        let interface1 = router1.interfaces.values().next().unwrap();
        let interface1_id = interface1.id;
        
        // R2のインターフェースを取得
        let router2 = sim.topology.routers.get(&r2).unwrap();
        let interface2 = router2.interfaces.values().next().unwrap();
        let interface2_id = interface2.id;
        
        console_log!("Testing authentication configuration on interfaces");
        
        // R1のインターフェースにシンプルパスワード認証を設定
        let auth_config1 = InterfaceConfig {
            ip_address: None,
            netmask: None,
            cost: None,
            hello_interval: None,
            dead_interval: None,
            priority: None,
            mtu: None,
            enabled: None,
            auth_type: Some(AuthType::SimplePassword),
            auth_key: Some("test123".to_string()),
            auth_key_id: None,
            inf_trans_delay: Some(1),
            rxmt_interval: Some(5),
        };
        
        sim.update_interface_config(r1, interface1_id, auth_config1).unwrap();
        
        // R2のインターフェースにも同じ認証を設定
        let auth_config2 = InterfaceConfig {
            ip_address: None,
            netmask: None,
            cost: None,
            hello_interval: None,
            dead_interval: None,
            priority: None,
            mtu: None,
            enabled: None,
            auth_type: Some(AuthType::SimplePassword),
            auth_key: Some("test123".to_string()),
            auth_key_id: None,
            inf_trans_delay: Some(1),
            rxmt_interval: Some(5),
        };
        
        sim.update_interface_config(r2, interface2_id, auth_config2).unwrap();
        
        // 設定が正しく適用されたことを確認
        let router1 = sim.topology.routers.get(&r1).unwrap();
        let interface1 = router1.interfaces.get(&interface1_id).unwrap();
        assert_eq!(interface1.auth_config.auth_type, AuthType::SimplePassword);
        assert_eq!(interface1.auth_config.auth_key, Some("test123".to_string()));
        
        let router2 = sim.topology.routers.get(&r2).unwrap();
        let interface2 = router2.interfaces.get(&interface2_id).unwrap();
        assert_eq!(interface2.auth_config.auth_type, AuthType::SimplePassword);
        assert_eq!(interface2.auth_config.auth_key, Some("test123".to_string()));
        
        console_log!("Authentication configuration test passed");
    }

    #[test]
    fn test_md5_auth_configuration() {
        let mut sim = NetworkSimulation::new();
        
        // 3つのルーターを作成（より複雑なトポロジー）
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        let r3 = sim.add_router("R3".to_string(), 50.0, 100.0);
        
        // 接続を作成
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_routers(r3, r1, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // 全インターフェースにMD5認証を設定
        for (router_id, router) in sim.topology.routers.clone() {
            for (interface_id, _interface) in router.interfaces {
                let md5_config = InterfaceConfig {
                    ip_address: None,
                    netmask: None,
                    cost: None,
                    hello_interval: None,
                    dead_interval: None,
                    priority: None,
                    mtu: None,
                    enabled: None,
                    auth_type: Some(AuthType::CryptographicMD5),
                    auth_key: Some("md5secret".to_string()),
                    auth_key_id: Some(1),
                    inf_trans_delay: Some(1),
                    rxmt_interval: Some(5),
                };
                
                sim.update_interface_config(router_id, interface_id, md5_config).unwrap();
            }
        }
        
        // 設定が正しく適用されたことを確認
        for router in sim.topology.routers.values() {
            for interface in router.interfaces.values() {
                assert_eq!(interface.auth_config.auth_type, AuthType::CryptographicMD5);
                assert_eq!(interface.auth_config.auth_key, Some("md5secret".to_string()));
                assert_eq!(interface.auth_config.key_id, Some(1));
            }
        }
        
        console_log!("MD5 authentication configuration test passed");
    }

    #[test]
    fn test_mixed_auth_configuration() {
        let mut sim = NetworkSimulation::new();
        
        // ハブアンドスポーク構成
        let hub = sim.add_router("HUB".to_string(), 50.0, 50.0);
        let spoke1 = sim.add_router("SPOKE1".to_string(), 0.0, 0.0);
        let spoke2 = sim.add_router("SPOKE2".to_string(), 100.0, 0.0);
        let spoke3 = sim.add_router("SPOKE3".to_string(), 0.0, 100.0);
        
        // 接続を作成
        sim.connect_routers(hub, spoke1, 10).unwrap();
        sim.connect_routers(hub, spoke2, 10).unwrap();
        sim.connect_routers(hub, spoke3, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(hub).unwrap();
        sim.enable_ospf(spoke1).unwrap();
        sim.enable_ospf(spoke2).unwrap();
        sim.enable_ospf(spoke3).unwrap();
        
        // HUB-SPOKE1リンクは認証なし
        // HUB-SPOKE2リンクはシンプルパスワード
        // HUB-SPOKE3リンクはMD5
        
        // 各インターフェースを特定して設定
        let (spoke1_interface_id, spoke2_interface_id, spoke3_interface_id) = {
            let hub_router = sim.topology.routers.get(&hub).unwrap();
            let hub_interfaces: Vec<_> = hub_router.interfaces.iter().collect();
            
            // インターフェースを接続先ルーターIDでソート（予測可能な順序のため）
            let mut sorted_interfaces = hub_interfaces;
            sorted_interfaces.sort_by_key(|(_, iface)| iface.connected_router_id);
            
            (*sorted_interfaces[0].0, *sorted_interfaces[1].0, *sorted_interfaces[2].0)
        };
        
        // SPOKE1への接続（認証なし）
        let null_config = InterfaceConfig {
            ip_address: None,
            netmask: None,
            cost: None,
            hello_interval: None,
            dead_interval: None,
            priority: None,
            mtu: None,
            enabled: None,
            auth_type: Some(AuthType::Null),
            auth_key: None,
            auth_key_id: None,
            inf_trans_delay: Some(1),
            rxmt_interval: Some(5),
        };
        sim.update_interface_config(hub, spoke1_interface_id, null_config).unwrap();
        
        // SPOKE2への接続（シンプルパスワード）
        let simple_config = InterfaceConfig {
            ip_address: None,
            netmask: None,
            cost: None,
            hello_interval: None,
            dead_interval: None,
            priority: None,
            mtu: None,
            enabled: None,
            auth_type: Some(AuthType::SimplePassword),
            auth_key: Some("spoke2pass".to_string()),
            auth_key_id: None,
            inf_trans_delay: Some(1),
            rxmt_interval: Some(5),
        };
        sim.update_interface_config(hub, spoke2_interface_id, simple_config).unwrap();
        
        // SPOKE3への接続（MD5）
        let md5_config = InterfaceConfig {
            ip_address: None,
            netmask: None,
            cost: None,
            hello_interval: None,
            dead_interval: None,
            priority: None,
            mtu: None,
            enabled: None,
            auth_type: Some(AuthType::CryptographicMD5),
            auth_key: Some("spoke3md5".to_string()),
            auth_key_id: Some(3),
            inf_trans_delay: Some(1),
            rxmt_interval: Some(5),
        };
        sim.update_interface_config(hub, spoke3_interface_id, md5_config).unwrap();
        
        // 設定を確認
        let hub_router = sim.topology.routers.get(&hub).unwrap();
        let spoke1_interface = hub_router.interfaces.get(&spoke1_interface_id).unwrap();
        assert_eq!(spoke1_interface.auth_config.auth_type, AuthType::Null);
        
        let spoke2_interface = hub_router.interfaces.get(&spoke2_interface_id).unwrap();
        assert_eq!(spoke2_interface.auth_config.auth_type, AuthType::SimplePassword);
        assert_eq!(spoke2_interface.auth_config.auth_key, Some("spoke2pass".to_string()));
        
        let spoke3_interface = hub_router.interfaces.get(&spoke3_interface_id).unwrap();
        assert_eq!(spoke3_interface.auth_config.auth_type, AuthType::CryptographicMD5);
        assert_eq!(spoke3_interface.auth_config.auth_key, Some("spoke3md5".to_string()));
        assert_eq!(spoke3_interface.auth_config.key_id, Some(3));
        
        console_log!("Mixed authentication configuration test passed");
    }
}