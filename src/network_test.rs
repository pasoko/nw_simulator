#[cfg(test)]
mod tests {
    use crate::network::NetworkTopology;
    use crate::network_type::OSPFNetworkType;
    
    #[test]
    fn test_network_topology_creation() {
        let topology = NetworkTopology::new();
        
        assert!(topology.routers.is_empty());
        assert!(topology.links.is_empty());
    }
    
    #[test]
    fn test_add_router() {
        let mut topology = NetworkTopology::new();
        
        let id1 = topology.add_router("Router1".to_string());
        let id2 = topology.add_router("Router2".to_string());
        let id3 = topology.add_router("Router3".to_string());
        
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_eq!(topology.routers.len(), 3);
        assert_eq!(topology.routers.get(&1).unwrap().name, "Router1");
        assert_eq!(topology.routers.get(&2).unwrap().name, "Router2");
        assert_eq!(topology.routers.get(&3).unwrap().name, "Router3");
    }
    
    #[test]
    fn test_connect_routers_basic() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        
        let link_id = topology.connect_routers(r1, r2, 10).unwrap();
        
        assert_eq!(topology.links.len(), 1);
        let link = topology.links.get(&link_id).unwrap();
        assert_eq!(link.router1_id, r1);
        assert_eq!(link.router2_id, r2);
        assert_eq!(link.cost, 10);
        assert!(!link.is_failed);
        assert_eq!(link.bandwidth, 100_000_000);
        assert_eq!(link.delay, 10);
        
        // Check interfaces were created
        let router1 = topology.routers.get(&r1).unwrap();
        let router2 = topology.routers.get(&r2).unwrap();
        assert_eq!(router1.interfaces.len(), 1);
        assert_eq!(router2.interfaces.len(), 1);
        
        // Check interface properties
        let iface1 = router1.interfaces.values().next().unwrap();
        let iface2 = router2.interfaces.values().next().unwrap();
        assert_eq!(iface1.connected_router_id, Some(r2));
        assert_eq!(iface2.connected_router_id, Some(r1));
        assert_eq!(iface1.cost, 10);
        assert_eq!(iface2.cost, 10);
        assert!(iface1.enabled);
        assert!(iface2.enabled);
    }
    
    #[test]
    fn test_connect_nonexistent_routers() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        
        // Try to connect to non-existent router
        let result = topology.connect_routers(r1, 999, 10);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Router 999 not found");
        
        // Try to connect from non-existent router
        let result = topology.connect_routers(999, r1, 10);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Router 999 not found");
    }
    
    #[test]
    fn test_multiple_connections() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        let r3 = topology.add_router("Router3".to_string());
        let r4 = topology.add_router("Router4".to_string());
        
        topology.connect_routers(r1, r2, 10).unwrap();
        topology.connect_routers(r1, r3, 20).unwrap();
        topology.connect_routers(r2, r3, 5).unwrap();
        topology.connect_routers(r3, r4, 15).unwrap();
        
        assert_eq!(topology.links.len(), 4);
        
        // Check router 1 has 2 interfaces
        assert_eq!(topology.routers.get(&r1).unwrap().interfaces.len(), 2);
        
        // Check router 3 has 3 interfaces (connected to r1, r2, and r4)
        assert_eq!(topology.routers.get(&r3).unwrap().interfaces.len(), 3);
    }
    
    #[test]
    fn test_get_neighbors() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        let r3 = topology.add_router("Router3".to_string());
        let r4 = topology.add_router("Router4".to_string());
        
        topology.connect_routers(r1, r2, 10).unwrap();
        topology.connect_routers(r1, r3, 20).unwrap();
        topology.connect_routers(r2, r3, 5).unwrap();
        
        let r1_neighbors = topology.get_neighbors(r1);
        let r2_neighbors = topology.get_neighbors(r2);
        let r3_neighbors = topology.get_neighbors(r3);
        let r4_neighbors = topology.get_neighbors(r4);
        
        assert_eq!(r1_neighbors.len(), 2);
        assert!(r1_neighbors.contains(&r2));
        assert!(r1_neighbors.contains(&r3));
        
        assert_eq!(r2_neighbors.len(), 2);
        assert!(r2_neighbors.contains(&r1));
        assert!(r2_neighbors.contains(&r3));
        
        assert_eq!(r3_neighbors.len(), 2);
        assert!(r3_neighbors.contains(&r1));
        assert!(r3_neighbors.contains(&r2));
        
        assert_eq!(r4_neighbors.len(), 0);
    }
    
    #[test]
    fn test_enable_ospf() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        
        assert!(topology.routers.get(&r1).unwrap().ospf_state.is_none());
        
        topology.enable_ospf_on_router(r1).unwrap();
        
        let router = topology.routers.get(&r1).unwrap();
        assert!(router.ospf_state.is_some());
        let ospf_state = router.ospf_state.as_ref().unwrap();
        assert_eq!(ospf_state.router_id, "1.1.1.1");
        assert_eq!(ospf_state.area_id, "0.0.0.0");
    }
    
    #[test]
    fn test_enable_ospf_nonexistent_router() {
        let mut topology = NetworkTopology::new();
        
        let result = topology.enable_ospf_on_router(999);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Router 999 not found");
    }
    
    #[test]
    fn test_ip_address_assignment() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        let r3 = topology.add_router("Router3".to_string());
        
        let link1_id = topology.connect_routers(r1, r2, 10).unwrap();
        let link2_id = topology.connect_routers(r2, r3, 20).unwrap();
        
        // Check IP addresses follow the pattern
        let link1 = topology.links.get(&link1_id).unwrap();
        let router1 = topology.routers.get(&r1).unwrap();
        let iface1 = router1.interfaces.get(&link1.router1_interface_id).unwrap();
        assert_eq!(iface1.ip_address, format!("10.0.{}.1", link1_id));
        
        let router2 = topology.routers.get(&r2).unwrap();
        let iface2 = router2.interfaces.get(&link1.router2_interface_id).unwrap();
        assert_eq!(iface2.ip_address, format!("10.0.{}.2", link1_id));
        
        // Check second link has different subnet
        let link2 = topology.links.get(&link2_id).unwrap();
        let iface3 = router2.interfaces.get(&link2.router1_interface_id).unwrap();
        assert_eq!(iface3.ip_address, format!("10.0.{}.1", link2_id));
    }
    
    #[test]
    fn test_link_failure_simulation() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        
        let link_id = topology.connect_routers(r1, r2, 10).unwrap();
        
        // Initially link should not be failed
        assert!(!topology.links.get(&link_id).unwrap().is_failed);
        
        // Simulate link failure
        if let Some(link) = topology.links.get_mut(&link_id) {
            link.is_failed = true;
        }
        
        assert!(topology.links.get(&link_id).unwrap().is_failed);
    }
    
    #[test]
    fn test_large_topology() {
        let mut topology = NetworkTopology::new();
        
        // Create 20 routers
        let mut router_ids = Vec::new();
        for i in 1..=20 {
            let id = topology.add_router(format!("Router{}", i));
            router_ids.push(id);
        }
        
        // Create a ring topology
        for i in 0..20 {
            let next = (i + 1) % 20;
            topology.connect_routers(router_ids[i], router_ids[next], 10).unwrap();
        }
        
        assert_eq!(topology.routers.len(), 20);
        assert_eq!(topology.links.len(), 20);
        
        // Each router should have exactly 2 neighbors
        for id in &router_ids {
            assert_eq!(topology.get_neighbors(*id).len(), 2);
        }
    }
    
    #[test]
    fn test_network_type_auto_detection() {
        let mut topology = NetworkTopology::new();
        
        // Test 1: First connection between two routers should be Point-to-Point
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        
        let link_id = topology.connect_routers(r1, r2, 10).unwrap();
        let link = topology.links.get(&link_id).unwrap();
        assert_eq!(link.network_type, OSPFNetworkType::PointToPoint);
        
        // Test 2: When adding a third router, new links should be Broadcast
        let r3 = topology.add_router("Router3".to_string());
        let link2_id = topology.connect_routers(r1, r3, 10).unwrap();
        let link2 = topology.links.get(&link2_id).unwrap();
        assert_eq!(link2.network_type, OSPFNetworkType::Broadcast);
    }
    
    #[test]
    fn test_network_type_explicit() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        
        // Explicitly set NBMA type
        let link_id = topology.connect_routers_with_type(
            r1, r2, 10, Some(OSPFNetworkType::NBMA)
        ).unwrap();
        
        let link = topology.links.get(&link_id).unwrap();
        assert_eq!(link.network_type, OSPFNetworkType::NBMA);
    }
    
    #[test]
    fn test_network_masks_by_type() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        let r3 = topology.add_router("Router3".to_string());
        let r4 = topology.add_router("Router4".to_string());
        
        // Create different network types
        let p2p_link = topology.connect_routers_with_type(
            r1, r2, 10, Some(OSPFNetworkType::PointToPoint)
        ).unwrap();
        
        let broadcast_link = topology.connect_routers_with_type(
            r3, r4, 10, Some(OSPFNetworkType::Broadcast)
        ).unwrap();
        
        // Check Point-to-Point uses /30 mask
        let p2p = topology.links.get(&p2p_link).unwrap();
        let r1_iface = topology.routers.get(&r1).unwrap()
            .interfaces.get(&p2p.router1_interface_id).unwrap();
        assert_eq!(r1_iface.netmask, "255.255.255.252");
        
        // Check Broadcast uses /24 mask
        let bcast = topology.links.get(&broadcast_link).unwrap();
        let r3_iface = topology.routers.get(&r3).unwrap()
            .interfaces.get(&bcast.router1_interface_id).unwrap();
        assert_eq!(r3_iface.netmask, "255.255.255.0");
    }
    
    #[test]
    fn test_bandwidth_and_delay() {
        let mut topology = NetworkTopology::new();
        
        let r1 = topology.add_router("Router1".to_string());
        let r2 = topology.add_router("Router2".to_string());
        
        let link_id = topology.connect_routers(r1, r2, 10).unwrap();
        let link = topology.links.get(&link_id).unwrap();
        
        assert_eq!(link.bandwidth, 100_000_000); // 100 Mbps
        assert_eq!(link.delay, 10); // 10ms
    }
}