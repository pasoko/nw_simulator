#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::router::InterfaceConfig;
    use crate::ospf_auth::AuthType;
    use crate::console_log;

    #[test]
    fn test_hello_packet_with_simple_password() {
        let mut sim = NetworkSimulation::new();
        
        // 2つのルーターを作成
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // ルーターを接続
        sim.topology.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // R1のインターフェースに簡易パスワード認証を設定
        if let Some(router) = sim.topology.routers.get(&r1) {
            let interface_id = router.interfaces.keys().next().copied().unwrap();
            let config = InterfaceConfig {
                ip_address: Some(router.interfaces[&interface_id].ip_address.clone()),
                netmask: Some(router.interfaces[&interface_id].netmask.clone()),
                enabled: Some(true),
                hello_interval: Some(10),
                dead_interval: Some(40),
                priority: Some(1),
                cost: Some(10),
                mtu: Some(1500),
                auth_type: Some(AuthType::SimplePassword),
                auth_key: Some("testpass".to_string()),
                auth_key_id: None,
                inf_trans_delay: Some(1),
                rxmt_interval: Some(5),
            };
            sim.update_interface_config(r1, interface_id, config).unwrap();
        }
        
        // R2のインターフェースに同じ簡易パスワード認証を設定
        if let Some(router) = sim.topology.routers.get(&r2) {
            let interface_id = router.interfaces.keys().next().copied().unwrap();
            let config = InterfaceConfig {
                ip_address: Some(router.interfaces[&interface_id].ip_address.clone()),
                netmask: Some(router.interfaces[&interface_id].netmask.clone()),
                enabled: Some(true),
                hello_interval: Some(10),
                dead_interval: Some(40),
                priority: Some(1),
                cost: Some(10),
                mtu: Some(1500),
                auth_type: Some(AuthType::SimplePassword),
                auth_key: Some("testpass".to_string()),
                auth_key_id: None,
                inf_trans_delay: Some(1),
                rxmt_interval: Some(5),
            };
            sim.update_interface_config(r2, interface_id, config).unwrap();
        }
        
        // シミュレーションを開始
        sim.start_simulation();
        
        // Helloパケット交換のために時間を進める
        for _ in 0..15 {
            sim.step_simulation(1.0);
        }
        
        // 両ルーターがneighbor関係を確立していることを確認
        let mut neighbors_established = false;
        
        if let Some(engine) = sim.get_ospf_engine(r1) {
            if engine.get_neighbor_count() > 0 {
                console_log!("R1 has {} neighbors", engine.get_neighbor_count());
                neighbors_established = true;
            }
        }
        
        assert!(neighbors_established, "Neighbors should be established with simple password authentication");
    }
    
    #[test]
    fn test_authentication_mismatch() {
        let mut sim = NetworkSimulation::new();
        
        // 2つのルーターを作成
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // ルーターを接続
        sim.topology.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // R1のインターフェースに簡易パスワード認証を設定
        if let Some(router) = sim.topology.routers.get(&r1) {
            let interface_id = router.interfaces.keys().next().copied().unwrap();
            let config = InterfaceConfig {
                ip_address: Some(router.interfaces[&interface_id].ip_address.clone()),
                netmask: Some(router.interfaces[&interface_id].netmask.clone()),
                enabled: Some(true),
                hello_interval: Some(10),
                dead_interval: Some(40),
                priority: Some(1),
                cost: Some(10),
                mtu: Some(1500),
                auth_type: Some(AuthType::SimplePassword),
                auth_key: Some("password1".to_string()),
                auth_key_id: None,
                inf_trans_delay: Some(1),
                rxmt_interval: Some(5),
            };
            sim.update_interface_config(r1, interface_id, config).unwrap();
        }
        
        // R2のインターフェースに異なるパスワードを設定
        if let Some(router) = sim.topology.routers.get(&r2) {
            let interface_id = router.interfaces.keys().next().copied().unwrap();
            let config = InterfaceConfig {
                ip_address: Some(router.interfaces[&interface_id].ip_address.clone()),
                netmask: Some(router.interfaces[&interface_id].netmask.clone()),
                enabled: Some(true),
                hello_interval: Some(10),
                dead_interval: Some(40),
                priority: Some(1),
                cost: Some(10),
                mtu: Some(1500),
                auth_type: Some(AuthType::SimplePassword),
                auth_key: Some("password2".to_string()),  // 異なるパスワード
                auth_key_id: None,
                inf_trans_delay: Some(1),
                rxmt_interval: Some(5),
            };
            sim.update_interface_config(r2, interface_id, config).unwrap();
        }
        
        // シミュレーションを開始
        sim.start_simulation();
        
        // Helloパケット交換のために時間を進める
        for _ in 0..15 {
            sim.step_simulation(1.0);
        }
        
        // 認証不一致のため、neighbor関係が確立されないことを確認
        let mut neighbors_established = false;
        
        if let Some(engine) = sim.get_ospf_engine(r1) {
            if engine.get_neighbor_count() > 0 {
                // Since we only have one neighbor (r2), we can check its state directly
                if let Some(state) = engine.get_neighbor_state(r2) {
                    if state as u8 > crate::router::OSPFNeighborState::Init as u8 {
                        neighbors_established = true;
                    }
                }
            }
        }
        
        assert!(!neighbors_established, "Neighbors should not progress beyond Init state with authentication mismatch");
    }
    
    #[test]
    fn test_md5_authentication() {
        let mut sim = NetworkSimulation::new();
        
        // 2つのルーターを作成
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // ルーターを接続
        sim.topology.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // R1のインターフェースにMD5認証を設定
        if let Some(router) = sim.topology.routers.get(&r1) {
            let interface_id = router.interfaces.keys().next().copied().unwrap();
            let config = InterfaceConfig {
                ip_address: Some(router.interfaces[&interface_id].ip_address.clone()),
                netmask: Some(router.interfaces[&interface_id].netmask.clone()),
                enabled: Some(true),
                hello_interval: Some(10),
                dead_interval: Some(40),
                priority: Some(1),
                cost: Some(10),
                mtu: Some(1500),
                auth_type: Some(AuthType::CryptographicMD5),
                auth_key: Some("md5secret".to_string()),
                auth_key_id: Some(1),
                inf_trans_delay: Some(1),
                rxmt_interval: Some(5),
            };
            sim.update_interface_config(r1, interface_id, config).unwrap();
        }
        
        // R2のインターフェースに同じMD5認証を設定
        if let Some(router) = sim.topology.routers.get(&r2) {
            let interface_id = router.interfaces.keys().next().copied().unwrap();
            let config = InterfaceConfig {
                ip_address: Some(router.interfaces[&interface_id].ip_address.clone()),
                netmask: Some(router.interfaces[&interface_id].netmask.clone()),
                enabled: Some(true),
                hello_interval: Some(10),
                dead_interval: Some(40),
                priority: Some(1),
                cost: Some(10),
                mtu: Some(1500),
                auth_type: Some(AuthType::CryptographicMD5),
                auth_key: Some("md5secret".to_string()),
                auth_key_id: Some(1),
                inf_trans_delay: Some(1),
                rxmt_interval: Some(5),
            };
            sim.update_interface_config(r2, interface_id, config).unwrap();
        }
        
        // シミュレーションを開始
        sim.start_simulation();
        
        // Helloパケット交換のために時間を進める
        for _ in 0..15 {
            sim.step_simulation(1.0);
        }
        
        // 両ルーターがneighbor関係を確立していることを確認
        let mut neighbors_established = false;
        
        if let Some(engine) = sim.get_ospf_engine(r1) {
            if engine.get_neighbor_count() > 0 {
                console_log!("R1 has {} neighbors with MD5 authentication", engine.get_neighbor_count());
                neighbors_established = true;
            }
        }
        
        assert!(neighbors_established, "Neighbors should be established with MD5 authentication");
    }
}