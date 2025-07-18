#[cfg(test)]
mod tests {
    use crate::router::*;

    #[test]
    fn test_router_state_creation() {
        let router = RouterState::new(1, "TestRouter".to_string());
        
        assert_eq!(router.id, 1);
        assert_eq!(router.name, "TestRouter");
        assert!(router.interfaces.is_empty());
        assert!(router.routing_table.is_empty());
        assert!(router.ospf_state.is_none());
        assert!(!router.is_failed);
    }

    #[test]
    fn test_interface_management() {
        let mut router = RouterState::new(1, "Router1".to_string());
        
        let interface1 = RouterInterface {
            id: 1,
            name: String::new(),
            ip_address: "192.168.1.1".to_string(),
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
            inf_trans_delay: 1,
            rxmt_interval: 5,
        };
        
        let interface2 = RouterInterface {
            id: 2,
            name: String::new(),
            ip_address: "10.0.0.1".to_string(),
            netmask: "255.255.255.0".to_string(),
            connected_router_id: None,
            cost: 1,
            enabled: false,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            mtu: 1500,
            manual_config: false,
            auth_config: crate::ospf_auth::AuthConfig::default(),
            inf_trans_delay: 1,
            rxmt_interval: 5,
        };
        
        router.add_interface(interface1.clone());
        router.add_interface(interface2.clone());
        
        assert_eq!(router.interfaces.len(), 2);
        assert_eq!(router.interfaces.get(&1).unwrap().ip_address, "192.168.1.1");
        assert_eq!(router.interfaces.get(&2).unwrap().enabled, false);
    }

    #[test]
    fn test_interface_replacement() {
        let mut router = RouterState::new(1, "Router1".to_string());
        
        let interface1 = RouterInterface {
            id: 1,
            name: String::new(),
            ip_address: "192.168.1.1".to_string(),
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
            inf_trans_delay: 1,
            rxmt_interval: 5,
        };
        
        router.add_interface(interface1);
        
        // Replace with updated interface
        let interface1_updated = RouterInterface {
            id: 1,
            name: String::new(),
            ip_address: "192.168.1.100".to_string(),
            netmask: "255.255.255.0".to_string(),
            connected_router_id: Some(2),
            cost: 20,
            enabled: false,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            mtu: 1500,
            manual_config: false,
            auth_config: crate::ospf_auth::AuthConfig::default(),
            inf_trans_delay: 1,
            rxmt_interval: 5,
        };
        
        router.add_interface(interface1_updated);
        
        assert_eq!(router.interfaces.len(), 1);
        assert_eq!(router.interfaces.get(&1).unwrap().ip_address, "192.168.1.100");
        assert_eq!(router.interfaces.get(&1).unwrap().cost, 20);
        assert!(!router.interfaces.get(&1).unwrap().enabled);
    }

    #[test]
    fn test_ospf_enablement() {
        let mut router = RouterState::new(1, "Router1".to_string());
        
        assert!(router.ospf_state.is_none());
        
        router.enable_ospf("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        assert!(router.ospf_state.is_some());
        let ospf_state = router.ospf_state.as_ref().unwrap();
        assert_eq!(ospf_state.router_id, "1.1.1.1");
        assert_eq!(ospf_state.area_id, "0.0.0.0");
        assert!(ospf_state.neighbors.is_empty());
        assert!(ospf_state.lsa_database.is_empty());
    }

    #[test]
    fn test_routing_table_management() {
        let mut router = RouterState::new(1, "Router1".to_string());
        
        let entry1 = RoutingTableEntry {
            destination: "192.168.2.0".to_string(),
            netmask: "255.255.255.0".to_string(),
            next_hop: "192.168.1.2".to_string(),
            interface_id: 1,
            interface_name: "IFRouter1-1".to_string(),
            metric: 10,
            protocol: RoutingProtocol::OSPF,
        };
        
        let entry2 = RoutingTableEntry {
            destination: "10.0.0.0".to_string(),
            netmask: "255.0.0.0".to_string(),
            next_hop: "192.168.1.2".to_string(),
            interface_id: 1,
            interface_name: "IFRouter1-1".to_string(),
            metric: 20,
            protocol: RoutingProtocol::Static,
        };
        
        router.update_routing_table(entry1.clone());
        router.update_routing_table(entry2.clone());
        
        assert_eq!(router.routing_table.len(), 2);
        assert_eq!(router.routing_table[0].destination, "192.168.2.0");
        assert_eq!(router.routing_table[1].destination, "10.0.0.0");
    }

    #[test]
    fn test_routing_table_update_same_destination() {
        let mut router = RouterState::new(1, "Router1".to_string());
        
        let entry1 = RoutingTableEntry {
            destination: "192.168.2.0".to_string(),
            netmask: "255.255.255.0".to_string(),
            next_hop: "192.168.1.2".to_string(),
            interface_id: 1,
            interface_name: "IFRouter1-1".to_string(),
            metric: 10,
            protocol: RoutingProtocol::OSPF,
        };
        
        router.update_routing_table(entry1);
        assert_eq!(router.routing_table.len(), 1);
        assert_eq!(router.routing_table[0].metric, 10);
        
        // Update with better metric
        let entry2 = RoutingTableEntry {
            destination: "192.168.2.0".to_string(),
            netmask: "255.255.255.0".to_string(),
            next_hop: "192.168.1.3".to_string(),
            interface_id: 2,
            interface_name: "IFRouter1-2".to_string(),
            metric: 5,
            protocol: RoutingProtocol::OSPF,
        };
        
        router.update_routing_table(entry2);
        assert_eq!(router.routing_table.len(), 1);
        assert_eq!(router.routing_table[0].metric, 5);
        assert_eq!(router.routing_table[0].next_hop, "192.168.1.3");
    }

    #[test]
    fn test_ospf_neighbor_states() {
        let neighbor = OSPFNeighbor {
            router_id: "2.2.2.2".to_string(),
            state: OSPFNeighborState::Down,
            interface_id: 1,
            priority: 1,
            dead_interval: 40,
        };
        
        assert_eq!(neighbor.state, OSPFNeighborState::Down);
        
        // Test all neighbor states
        let states = vec![
            OSPFNeighborState::Down,
            OSPFNeighborState::Init,
            OSPFNeighborState::TwoWay,
            OSPFNeighborState::ExStart,
            OSPFNeighborState::Exchange,
            OSPFNeighborState::Loading,
            OSPFNeighborState::Full,
        ];
        
        for state in states {
            let mut neighbor_copy = neighbor.clone();
            neighbor_copy.state = state.clone();
            assert_eq!(neighbor_copy.state, state);
        }
    }

    #[test]
    fn test_lsa_header_creation() {
        let header = LSAHeader {
            ls_age: 100,
            ls_type: LSAType::RouterLSA,
            link_state_id: "1.1.1.1".to_string(),
            advertising_router: "1.1.1.1".to_string(),
            ls_sequence_number: 0x80000001,
            ls_checksum: 0x1234,
            length: 36,
        };
        
        assert_eq!(header.ls_age, 100);
        assert_eq!(header.ls_type, LSAType::RouterLSA);
        assert_eq!(header.link_state_id, "1.1.1.1");
        assert_eq!(header.advertising_router, "1.1.1.1");
        assert_eq!(header.ls_sequence_number, 0x80000001);
        assert_eq!(header.ls_checksum, 0x1234);
        assert_eq!(header.length, 36);
    }

    #[test]
    fn test_router_lsa_creation() {
        let links = vec![
            RouterLink {
                link_id: "2.2.2.2".to_string(),
                link_data: "192.168.1.1".to_string(),
                link_type: LinkType::PointToPoint,
                num_tos: 0,
                metric: 10,
            },
            RouterLink {
                link_id: "192.168.2.0".to_string(),
                link_data: "255.255.255.0".to_string(),
                link_type: LinkType::StubNetwork,
                num_tos: 0,
                metric: 1,
            },
        ];
        
        let router_lsa = RouterLSA {
            flags: 0x01, // B bit set
            num_links: 2,
            links: links.clone(),
        };
        
        assert_eq!(router_lsa.flags, 0x01);
        assert_eq!(router_lsa.num_links, 2);
        assert_eq!(router_lsa.links.len(), 2);
        assert_eq!(router_lsa.links[0].link_type, LinkType::PointToPoint);
        assert_eq!(router_lsa.links[1].link_type, LinkType::StubNetwork);
    }

    #[test]
    fn test_network_lsa_creation() {
        let network_lsa = NetworkLSA {
            network_mask: "255.255.255.0".to_string(),
            attached_routers: vec![
                "1.1.1.1".to_string(),
                "2.2.2.2".to_string(),
                "3.3.3.3".to_string(),
            ],
        };
        
        assert_eq!(network_lsa.network_mask, "255.255.255.0");
        assert_eq!(network_lsa.attached_routers.len(), 3);
        assert_eq!(network_lsa.attached_routers[1], "2.2.2.2");
    }

    #[test]
    fn test_link_type_values() {
        assert_eq!(LinkType::PointToPoint as u8, 1);
        assert_eq!(LinkType::TransitNetwork as u8, 2);
        assert_eq!(LinkType::StubNetwork as u8, 3);
        assert_eq!(LinkType::VirtualLink as u8, 4);
    }

    #[test]
    fn test_lsa_type_values() {
        assert_eq!(LSAType::RouterLSA as u8, 1);
        assert_eq!(LSAType::NetworkLSA as u8, 2);
        assert_eq!(LSAType::SummaryLSA as u8, 3);
        assert_eq!(LSAType::SummaryASBR as u8, 4);
        assert_eq!(LSAType::ASExternalLSA as u8, 5);
    }

    #[test]
    fn test_router_failure_state() {
        let mut router = RouterState::new(1, "Router1".to_string());
        
        assert!(!router.is_failed);
        
        router.is_failed = true;
        assert!(router.is_failed);
        
        router.is_failed = false;
        assert!(!router.is_failed);
    }

    #[test]
    fn test_routing_protocol_equality() {
        assert_eq!(RoutingProtocol::Direct, RoutingProtocol::Direct);
        assert_eq!(RoutingProtocol::Static, RoutingProtocol::Static);
        assert_eq!(RoutingProtocol::OSPF, RoutingProtocol::OSPF);
        assert_ne!(RoutingProtocol::Direct, RoutingProtocol::OSPF);
    }

    #[test]
    fn test_complex_lsa_structure() {
        let header = LSAHeader {
            ls_age: 0,
            ls_type: LSAType::RouterLSA,
            link_state_id: "1.1.1.1".to_string(),
            advertising_router: "1.1.1.1".to_string(),
            ls_sequence_number: 0x80000001,
            ls_checksum: 0,
            length: 48,
        };
        
        let router_lsa = RouterLSA {
            flags: 0x02, // E bit set
            num_links: 3,
            links: vec![
                RouterLink {
                    link_id: "2.2.2.2".to_string(),
                    link_data: "192.168.1.1".to_string(),
                    link_type: LinkType::PointToPoint,
                    num_tos: 0,
                    metric: 10,
                },
                RouterLink {
                    link_id: "192.168.1.0".to_string(),
                    link_data: "255.255.255.0".to_string(),
                    link_type: LinkType::TransitNetwork,
                    num_tos: 0,
                    metric: 5,
                },
                RouterLink {
                    link_id: "10.0.0.0".to_string(),
                    link_data: "255.255.255.0".to_string(),
                    link_type: LinkType::StubNetwork,
                    num_tos: 0,
                    metric: 1,
                },
            ],
        };
        
        let lsa = LSA {
            header,
            data: LSAData::Router(router_lsa),
        };
        
        assert_eq!(lsa.header.ls_type, LSAType::RouterLSA);
        if let LSAData::Router(ref router) = lsa.data {
            assert_eq!(router.num_links, 3);
            assert_eq!(router.links[0].metric, 10);
            assert_eq!(router.links[1].link_type, LinkType::TransitNetwork);
            assert_eq!(router.links[2].link_id, "10.0.0.0");
        } else {
            panic!("Expected Router LSA");
        }
    }

    #[test]
    fn test_as_external_lsa() {
        let as_external = ASExternalLSA {
            network_mask: "255.255.255.0".to_string(),
            metric: 100,
            metric_type: 1, // Type2
            forwarding_address: "0.0.0.0".to_string(),
            external_route_tag: 0,
            tos: 0,
            tos_metric: 0,
        };
        
        let lsa = LSA {
            header: LSAHeader {
                ls_age: 0,
                ls_type: LSAType::ASExternalLSA,
                link_state_id: "192.168.0.0".to_string(),
                advertising_router: "1.1.1.1".to_string(),
                ls_sequence_number: 0x80000001,
                ls_checksum: 0,
                length: 36,
            },
            data: LSAData::ASExternal(as_external),
        };
        
        if let LSAData::ASExternal(ref external) = lsa.data {
            assert_eq!(external.metric, 100);
            assert_eq!(external.forwarding_address, "0.0.0.0");
            assert_eq!(external.external_route_tag, 0);
        } else {
            panic!("Expected AS External LSA");
        }
    }
}