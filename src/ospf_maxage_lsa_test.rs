#[cfg(test)]
mod tests {
    use crate::ospf_lsa_manager::OSPFLSAManager;
    use crate::router::{LSA, LSAHeader, LSAData, LSAType, RouterLSA};
    use crate::console_log;

    #[test]
    fn test_maxage_lsa_retention() {
        console_log!("=== Testing MaxAge LSA retention ===");
        
        let mut manager = OSPFLSAManager::new("1.1.1.1".to_string());
        
        // Create and add a test LSA
        let lsa = LSA {
            header: LSAHeader {
                ls_age: 100,
                ls_type: LSAType::RouterLSA,
                link_state_id: "1.1.1.2".to_string(),
                advertising_router: "1.1.1.2".to_string(),
                ls_sequence_number: 0x80000001,
                ls_checksum: 0,
                length: 0,
            },
            data: LSAData::Router(RouterLSA {
                flags: 0,
                num_links: 0,
                links: vec![],
            }),
        };
        
        manager.update_lsa_database(lsa.clone());
        assert_eq!(manager.get_lsa_count(), 1, "Should have 1 LSA initially");
        
        // Age the LSA to MaxAge (3600 seconds)
        manager.update_time(3600.0);
        let maxage_lsas = manager.age_lsas(3500.0);
        
        assert_eq!(maxage_lsas.len(), 1, "Should have 1 MaxAge LSA");
        assert_eq!(maxage_lsas[0].header.ls_age, 3600, "LSA should be MaxAge");
        
        // Verify LSA is still in database after reaching MaxAge
        assert_eq!(manager.get_lsa_count(), 1, "MaxAge LSA should still be in database");
        
        // Age further by 100 seconds
        manager.update_time(100.0);
        manager.age_lsas(0.0);
        
        // In the old implementation, the LSA would be removed after 60 seconds
        // In the new implementation, it should still be there
        assert_eq!(manager.get_lsa_count(), 1, "MaxAge LSA should be retained (OSPFv2 compliance)");
        
        console_log!("Test passed: MaxAge LSAs are properly retained");
    }
    
    #[test]
    fn test_unreachable_lsa_removal() {
        console_log!("=== Testing unreachable LSA removal ===");
        
        let mut manager = OSPFLSAManager::new("1.1.1.1".to_string());
        
        // Create LSAs from different routers
        for i in 2..=5 {
            let lsa = LSA {
                header: LSAHeader {
                    ls_age: 3600, // MaxAge
                    ls_type: LSAType::RouterLSA,
                    link_state_id: format!("1.1.1.{}", i),
                    advertising_router: format!("1.1.1.{}", i),
                    ls_sequence_number: 0x80000001,
                    ls_checksum: 0,
                    length: 0,
                },
                data: LSAData::Router(RouterLSA {
                    flags: 0,
                    num_links: 0,
                    links: vec![],
                }),
            };
            manager.update_lsa_database(lsa);
        }
        
        assert_eq!(manager.get_lsa_count(), 4, "Should have 4 LSAs");
        
        // Simulate SPF calculation result: only routers 2 and 3 are reachable
        let mut reachable_routers = std::collections::HashSet::new();
        reachable_routers.insert(1); // self
        reachable_routers.insert(2);
        reachable_routers.insert(3);
        
        // Remove unreachable LSAs
        manager.remove_unreachable_lsas(&reachable_routers);
        
        // Should remove LSAs from routers 4 and 5
        assert_eq!(manager.get_lsa_count(), 2, "Should have removed unreachable LSAs");
        
        // Verify correct LSAs remain
        let db = manager.get_lsa_database();
        for (_, lsa) in db {
            let router_id = lsa.header.advertising_router
                .split('.')
                .last()
                .unwrap()
                .parse::<u32>()
                .unwrap();
            assert!(router_id == 2 || router_id == 3, 
                "Only LSAs from routers 2 and 3 should remain");
        }
        
        console_log!("Test passed: Unreachable LSAs are properly removed");
    }
}