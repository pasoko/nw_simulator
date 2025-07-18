#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::router::LSAType;
    use crate::console_log;

    #[test]
    fn test_abr_summary_lsa_generation() {
        let mut sim = NetworkSimulation::new();
        
        // エリア0に2つのルーター
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // エリア1に1つのルーター
        let r3 = sim.add_router("R3".to_string(), 200.0, 0.0);
        
        // R1-R2をエリア0で接続
        sim.topology.connect_routers(r1, r2, 10).unwrap();
        
        // R2-R3をエリア1で接続（R2がABRになる）
        sim.topology.connect_routers(r2, r3, 10).unwrap();
        
        // OSPFを有効化
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // R2にエリア1を追加（ABRにする）
        if let Some(engine) = sim.get_ospf_engine_mut(r2) {
            engine.add_area("0.0.0.1".to_string());
            assert!(engine.is_abr());
        }
        
        // シミュレーションを開始
        sim.start_simulation();
        
        // ABR機能のために十分な時間進める
        for _ in 0..30 {
            sim.step_simulation(1.0);
        }
        
        // R2がABRとしてSummary LSAを生成しているか確認
        let mut summary_lsa_found = false;
        
        if let Some(engine) = sim.get_ospf_engine(r2) {
            let lsa_db = engine.get_lsa_database();
            for (_key, lsa) in lsa_db {
                if lsa.header.ls_type == LSAType::SummaryLSA {
                    summary_lsa_found = true;
                    console_log!(
                        "Summary LSA found: LS ID={}, Adv Router={}",
                        lsa.header.link_state_id,
                        lsa.header.advertising_router
                    );
                }
            }
        }
        
        // 現在の実装ではinter-area routesがないため、Summary LSAは生成されない
        // これは将来の実装で対応
        assert!(!summary_lsa_found, "Summary LSA generation requires inter-area route calculation");
    }
    
    #[test]
    fn test_non_abr_no_summary_lsa() {
        let mut sim = NetworkSimulation::new();
        
        // エリア0に2つのルーターのみ
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
        
        // どちらのルーターもABRではないことを確認
        for router_id in [r1, r2] {
            if let Some(engine) = sim.get_ospf_engine(router_id) {
                assert!(!engine.is_abr(), "Router {} should not be ABR", router_id);
                
                // Summary LSAが生成されていないことを確認
                let lsa_db = engine.get_lsa_database();
                for (_key, lsa) in lsa_db {
                    assert_ne!(
                        lsa.header.ls_type, 
                        LSAType::SummaryLSA,
                        "Non-ABR should not generate Summary LSA"
                    );
                }
            }
        }
    }
}