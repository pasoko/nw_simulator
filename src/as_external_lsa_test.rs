#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::router::LSAType;
    use crate::as_external_lsa::ExternalMetricType;
    use crate::console_log;

    #[test]
    fn test_asbr_as_external_lsa_generation() {
        let mut sim = NetworkSimulation::new();
        
        // OSPFドメイン内のルーター
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        sim.topology.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // R2に外部ルートを追加（ASBRにする）
        if let Some(engine) = sim.get_ospf_engine_mut(r2) {
            // デフォルトルートを追加
            engine.add_external_route(
                "0.0.0.0".to_string(),
                "0.0.0.0".to_string(),
                1,
                ExternalMetricType::Type2,
                "0.0.0.0".to_string(),
                0,
            );
            
            // 外部ネットワークを追加
            engine.add_external_route(
                "10.0.0.0".to_string(),
                "255.0.0.0".to_string(),
                100,
                ExternalMetricType::Type1,
                "192.168.1.1".to_string(),
                12345,
            );
            
            assert!(engine.is_asbr());
        }
        
        // シミュレーションを開始
        sim.start_simulation();
        
        // ASBR機能のために十分な時間進める
        for _ in 0..20 {
            sim.step_simulation(1.0);
        }
        
        // R2がASBRとしてAS-External LSAを生成する
        if let Some(engine) = sim.get_ospf_engine_mut(r2) {
            let lsas = engine.generate_as_external_lsas();
            assert_eq!(lsas.len(), 2, "ASBR should generate 2 AS-External LSAs");
            
            // 生成されたLSAを確認
            for lsa in &lsas {
                assert_eq!(lsa.header.ls_type, LSAType::ASExternalLSA);
                console_log!(
                    "AS-External LSA generated: LS ID={}, Adv Router={}",
                    lsa.header.link_state_id,
                    lsa.header.advertising_router
                );
                
                if let crate::router::LSAData::ASExternal(ref ext_lsa) = lsa.data {
                    console_log!(
                        "AS-External LSA content: mask={}, metric={}, type={}, fwd_addr={}, tag={}",
                        ext_lsa.network_mask,
                        ext_lsa.metric,
                        ext_lsa.metric_type,
                        ext_lsa.forwarding_address,
                        ext_lsa.external_route_tag
                    );
                }
            }
        }
        
        // LSAデータベースを確認
        let mut as_external_lsa_count = 0;
        
        if let Some(engine) = sim.get_ospf_engine(r2) {
            let lsa_db = engine.get_lsa_database();
            for (_key, lsa) in lsa_db {
                if lsa.header.ls_type == LSAType::ASExternalLSA {
                    as_external_lsa_count += 1;
                }
            }
        }
        
        assert_eq!(as_external_lsa_count, 2, "LSA database should contain 2 AS-External LSAs");
    }
    
    #[test]
    fn test_non_asbr_no_as_external_lsa() {
        let mut sim = NetworkSimulation::new();
        
        // OSPFドメイン内のルーターのみ
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        sim.topology.connect_routers(r1, r2, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // シミュレーションを開始
        sim.start_simulation();
        
        // 十分な時間進める
        for _ in 0..20 {
            sim.step_simulation(1.0);
        }
        
        // どちらのルーターもASBRではないことを確認
        for router_id in [r1, r2] {
            if let Some(engine) = sim.get_ospf_engine(router_id) {
                assert!(!engine.is_asbr(), "Router {} should not be ASBR", router_id);
                
                // AS-External LSAが生成されていないことを確認
                let lsa_db = engine.get_lsa_database();
                for (_key, lsa) in lsa_db {
                    assert_ne!(
                        lsa.header.ls_type, 
                        LSAType::ASExternalLSA,
                        "Non-ASBR should not have AS-External LSA"
                    );
                }
            }
        }
    }
    
    #[test]
    fn test_default_route_as_external_lsa() {
        let mut sim = NetworkSimulation::new();
        
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        
        // デフォルトルートのみを追加
        if let Some(engine) = sim.get_ospf_engine_mut(r1) {
            engine.add_external_route(
                "0.0.0.0".to_string(),
                "0.0.0.0".to_string(),
                1,
                ExternalMetricType::Type2,
                "0.0.0.0".to_string(),
                0,
            );
            
            let lsas = engine.generate_as_external_lsas();
            assert_eq!(lsas.len(), 1, "Should generate 1 AS-External LSA for default route");
            
            let default_lsa = &lsas[0];
            assert_eq!(default_lsa.header.link_state_id, "0.0.0.0");
            
            if let crate::router::LSAData::ASExternal(ref ext_lsa) = default_lsa.data {
                assert_eq!(ext_lsa.network_mask, "0.0.0.0");
                assert_eq!(ext_lsa.metric, 1);
                assert_eq!(ext_lsa.metric_type, 1); // Type2
                assert_eq!(ext_lsa.forwarding_address, "0.0.0.0");
            }
        }
    }
}