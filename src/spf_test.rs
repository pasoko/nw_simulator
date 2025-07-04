#[cfg(test)]
mod tests {
    use crate::spf::SPFCalculator;
    use crate::router::{LSA, LSAHeader, LSAData, RouterLSA, RouterLink, LinkType, LSAType, RoutingProtocol};
    use crate::network::NetworkTopology;
    use std::collections::HashMap;

    fn create_test_lsa(router_id: u32, links: Vec<(u32, u16)>) -> LSA {
        let router_links: Vec<RouterLink> = links
            .into_iter()
            .map(|(neighbor_id, metric)| RouterLink {
                link_id: format!("1.1.1.{}", neighbor_id),
                link_type: LinkType::PointToPoint,
                metric,
                link_data: format!("255.255.255.{}", 252),
                num_tos: 0,
            })
            .collect();

        LSA {
            header: LSAHeader {
                ls_age: 0,
                ls_type: LSAType::RouterLSA,
                link_state_id: format!("1.1.1.{}", router_id),
                advertising_router: format!("1.1.1.{}", router_id),
                ls_sequence_number: 0x80000001,
                ls_checksum: 0,
                length: 0,
            },
            data: LSAData::Router(RouterLSA {
                flags: 0,
                num_links: router_links.len() as u16,
                links: router_links,
            }),
        }
    }

    fn create_test_topology() -> NetworkTopology {
        let mut topology = NetworkTopology::new();
        
        // Add routers
        for i in 1..=4 {
            topology.add_router(format!("Router{}", i));
        }
        
        // Add links between routers
        topology.connect_routers(1, 2, 10).unwrap();
        topology.connect_routers(1, 3, 20).unwrap();
        topology.connect_routers(2, 3, 5).unwrap();
        topology.connect_routers(2, 4, 15).unwrap();
        topology.connect_routers(3, 4, 10).unwrap();
        
        topology
    }


    #[test]
    fn test_spf_empty_database() {
        let lsa_database = HashMap::new();
        let topology = NetworkTopology::new();
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        assert!(routes.is_empty());
    }

    #[test]
    fn test_spf_single_router() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        // Only router 1 in database
        let lsa1 = create_test_lsa(1, vec![]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Should have no routes (no neighbors)
        assert!(routes.is_empty());
    }

    #[test]
    fn test_spf_simple_topology() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        // Create LSAs for a simple 3-router topology
        // Router 1 connects to Router 2 (cost 10) and Router 3 (cost 20)
        let lsa1 = create_test_lsa(1, vec![(2, 10), (3, 20)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        // Router 2 connects to Router 1 (cost 10) and Router 3 (cost 5)
        let lsa2 = create_test_lsa(2, vec![(1, 10), (3, 5)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        // Router 3 connects to Router 1 (cost 20) and Router 2 (cost 5)
        let lsa3 = create_test_lsa(3, vec![(1, 20), (2, 5)]);
        lsa_database.insert("1:1.1.1.3:1.1.1.3".to_string(), lsa3);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Check that we have routes to both routers
        assert_eq!(routes.len(), 2);
        
        // Check route to router 2
        if let Some(route_to_2) = routes.get(&2) {
            assert_eq!(route_to_2.destination, "1.1.1.2");
            assert_eq!(route_to_2.metric, 10);
            assert_eq!(route_to_2.next_hop, "1.1.1.2");
        } else {
            panic!("No route to router 2");
        }
        
        // Check route to router 3 - should go via router 2 (cost 15)
        if let Some(route_to_3) = routes.get(&3) {
            assert_eq!(route_to_3.destination, "1.1.1.3");
            assert_eq!(route_to_3.metric, 15); // 10 + 5
            assert_eq!(route_to_3.next_hop, "1.1.1.2"); // Via router 2
        } else {
            panic!("No route to router 3");
        }
    }

    #[test]
    fn test_spf_complex_topology() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        // Create LSAs for all 4 routers
        let lsa1 = create_test_lsa(1, vec![(2, 10), (3, 20)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let lsa2 = create_test_lsa(2, vec![(1, 10), (3, 5), (4, 15)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        let lsa3 = create_test_lsa(3, vec![(1, 20), (2, 5), (4, 10)]);
        lsa_database.insert("1:1.1.1.3:1.1.1.3".to_string(), lsa3);
        
        let lsa4 = create_test_lsa(4, vec![(2, 15), (3, 10)]);
        lsa_database.insert("1:1.1.1.4:1.1.1.4".to_string(), lsa4);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Should have routes to all 3 other routers
        assert_eq!(routes.len(), 3);
        
        // Check optimal paths
        assert_eq!(routes.get(&2).unwrap().metric, 10);  // Direct path
        assert_eq!(routes.get(&3).unwrap().metric, 15);  // Via router 2
        assert_eq!(routes.get(&4).unwrap().metric, 25);  // Via router 2
    }

    #[test]
    fn test_spf_with_unreachable_router() {
        let mut lsa_database = HashMap::new();
        let mut topology = create_test_topology();
        
        // Create a disconnected router 5
        topology.add_router("Router5".to_string());
        
        // LSAs for connected component
        let lsa1 = create_test_lsa(1, vec![(2, 10)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let lsa2 = create_test_lsa(2, vec![(1, 10)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        // LSA for disconnected router
        let lsa5 = create_test_lsa(5, vec![]);
        lsa_database.insert("1:1.1.1.5:1.1.1.5".to_string(), lsa5);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Should only have route to router 2, not to router 5
        assert_eq!(routes.len(), 1);
        assert!(routes.contains_key(&2));
        assert!(!routes.contains_key(&5));
    }

    #[test]
    fn test_spf_routing_protocol() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        let lsa1 = create_test_lsa(1, vec![(2, 10)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let lsa2 = create_test_lsa(2, vec![(1, 10)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Check that routes are marked as OSPF protocol
        for (_, route) in routes {
            assert_eq!(route.protocol, RoutingProtocol::OSPF);
        }
    }

    #[test]
    fn test_spf_with_asymmetric_costs() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        // Create asymmetric costs: 1->2 cost 10, 2->1 cost 50
        let lsa1 = create_test_lsa(1, vec![(2, 10), (3, 30)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let lsa2 = create_test_lsa(2, vec![(1, 50), (3, 5)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        let lsa3 = create_test_lsa(3, vec![(1, 30), (2, 5)]);
        lsa_database.insert("1:1.1.1.3:1.1.1.3".to_string(), lsa3);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Path to router 3 should prefer 1->2->3 (cost 15) over direct 1->3 (cost 30)
        if let Some(route_to_3) = routes.get(&3) {
            assert_eq!(route_to_3.metric, 15);
            assert_eq!(route_to_3.next_hop, "1.1.1.2");
        }
    }

    #[test]
    fn test_spf_equal_cost_paths() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        // Create equal cost paths: 1->2->4 and 1->3->4 both cost 20
        let lsa1 = create_test_lsa(1, vec![(2, 10), (3, 10)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let lsa2 = create_test_lsa(2, vec![(1, 10), (4, 10)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        let lsa3 = create_test_lsa(3, vec![(1, 10), (4, 10)]);
        lsa_database.insert("1:1.1.1.3:1.1.1.3".to_string(), lsa3);
        
        let lsa4 = create_test_lsa(4, vec![(2, 10), (3, 10)]);
        lsa_database.insert("1:1.1.1.4:1.1.1.4".to_string(), lsa4);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Should find path to router 4 with cost 20
        if let Some(route_to_4) = routes.get(&4) {
            assert_eq!(route_to_4.metric, 20);
            // Next hop could be either router 2 or 3
            assert!(route_to_4.next_hop == "1.1.1.2" || route_to_4.next_hop == "1.1.1.3");
        }
    }

    #[test]
    fn test_spf_link_failure_reroute() {
        let mut lsa_database = HashMap::new();
        let mut topology = NetworkTopology::new();
        
        // Create simple 3-router topology
        topology.add_router("Router1".to_string());
        topology.add_router("Router2".to_string());
        topology.add_router("Router3".to_string());
        
        // Add links matching LSA costs
        topology.connect_routers(1, 2, 10).unwrap();
        topology.connect_routers(1, 3, 100).unwrap();
        topology.connect_routers(2, 3, 10).unwrap();
        
        // Initial topology with redundant paths
        let lsa1 = create_test_lsa(1, vec![(2, 10), (3, 100)]);
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let lsa2 = create_test_lsa(2, vec![(1, 10), (3, 10)]);
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        let lsa3 = create_test_lsa(3, vec![(1, 100), (2, 10)]);
        lsa_database.insert("1:1.1.1.3:1.1.1.3".to_string(), lsa3);
        
        // First calculation - should prefer 1->2->3
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        assert_eq!(routes.get(&3).unwrap().metric, 20);
        assert_eq!(routes.get(&3).unwrap().next_hop, "1.1.1.2");
        
        // Simulate link failure by marking link as failed
        for link in topology.links.values_mut() {
            if (link.router1_id == 1 && link.router2_id == 2) ||
               (link.router1_id == 2 && link.router2_id == 1) {
                link.is_failed = true;
            }
        }
        
        // Update LSAs to reflect the link failure (in real OSPF, routers would regenerate LSAs)
        // Remove the failed link from router 1's LSA
        let lsa1_updated = create_test_lsa(1, vec![(3, 100)]); // Only link to router 3 remains
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1_updated);
        
        // Remove the failed link from router 2's LSA
        let lsa2_updated = create_test_lsa(2, vec![(3, 10)]); // Only link to router 3 remains
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2_updated);
        
        // Recalculate - should now use direct path 1->3
        let routes_after_failure = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Check if we have a route to router 3
        assert!(routes_after_failure.contains_key(&3), "No route to router 3 after link failure");
        assert_eq!(routes_after_failure.get(&3).unwrap().metric, 100);
        assert_eq!(routes_after_failure.get(&3).unwrap().next_hop, "1.1.1.3");
    }

    #[test]
    fn test_spf_performance_large_network() {
        use std::time::Instant;
        
        let mut lsa_database = HashMap::new();
        let mut topology = NetworkTopology::new();
        
        // Create a large network with 100 routers
        for i in 1..=100 {
            topology.add_router(format!("Router{}", i));
        }
        
        // Create a mesh topology where each router connects to 5 others
        // Track created links to ensure bidirectional connections
        let mut created_links = std::collections::HashSet::new();
        
        for i in 1..=100 {
            let mut links = Vec::new();
            for j in 1..=5 {
                let target = ((i + j * 20 - 1) % 100) + 1;
                if target != i {
                    let link_key = if i < target { (i, target) } else { (target, i) };
                    
                    // Only create physical link once
                    if !created_links.contains(&link_key) {
                        topology.connect_routers(i as u32, target as u32, 10).ok();
                        created_links.insert(link_key);
                    }
                    
                    links.push((target as u32, 10));
                }
            }
            let lsa = create_test_lsa(i as u32, links);
            lsa_database.insert(format!("1:1.1.1.{}:1.1.1.{}", i, i), lsa);
        }
        
        // Measure SPF calculation time
        let start = Instant::now();
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        let duration = start.elapsed();
        
        // Should complete in reasonable time (< 100ms for 100 routers)
        assert!(duration.as_millis() < 100, "SPF took too long: {:?}", duration);
        
        // Should find routes to direct neighbors at minimum
        // Router 1 connects to routers: 21, 41, 61, 81, 1 (skip self)
        // So we expect at least 4 routes
        assert!(routes.len() >= 4, "Found too few routes: {}", routes.len());
    }

    #[test]
    fn test_spf_different_network_masks() {
        let mut lsa_database = HashMap::new();
        let topology = create_test_topology();
        
        // Create LSAs with different network configurations
        let mut lsa1 = create_test_lsa(1, vec![(2, 10)]);
        if let LSAData::Router(ref mut router_lsa) = lsa1.data {
            router_lsa.links[0].link_data = "255.255.255.0".to_string();
        }
        lsa_database.insert("1:1.1.1.1:1.1.1.1".to_string(), lsa1);
        
        let mut lsa2 = create_test_lsa(2, vec![(1, 10)]);
        if let LSAData::Router(ref mut router_lsa) = lsa2.data {
            router_lsa.links[0].link_data = "255.255.255.0".to_string();
        }
        lsa_database.insert("1:1.1.1.2:1.1.1.2".to_string(), lsa2);
        
        let routes = SPFCalculator::calculate_routes_from_lsa(&lsa_database, 1, &topology);
        
        // Should still calculate routes correctly
        assert_eq!(routes.len(), 1);
        assert_eq!(routes.get(&2).unwrap().netmask, "255.255.255.255");
    }
}