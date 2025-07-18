use crate::router::{LSA, LSAHeader, LSAData, LSAType, NetworkLSA};
use crate::console_log;
use std::collections::HashSet;

/// Network LSA Generator
/// 
/// RFC 2328 Section 12.4.2: Generating Network-LSAs
/// The Designated Router (DR) generates a Network LSA for each
/// multi-access network where it is the DR.
pub struct NetworkLSAGenerator {
    router_id: String,
}

impl NetworkLSAGenerator {
    pub fn new(router_id: String) -> Self {
        NetworkLSAGenerator {
            router_id,
        }
    }
    
    /// Generate a Network LSA for a broadcast/NBMA network
    /// 
    /// Parameters:
    /// - interface_ip: IP address of the DR's interface (used as Link State ID)
    /// - network_mask: Network mask of the broadcast domain
    /// - attached_routers: Set of Router IDs that are fully adjacent (including DR itself)
    /// - sequence_number: LSA sequence number
    pub fn generate_network_lsa(
        &self,
        interface_ip: &str,
        network_mask: &str,
        attached_routers: &HashSet<String>,
        sequence_number: u32,
    ) -> LSA {
        console_log!(
            "DR {} generating Network LSA for interface {} with {} attached routers",
            self.router_id, interface_ip, attached_routers.len()
        );
        
        // The Link State ID for Network LSA is the IP address of the DR's interface
        let link_state_id = interface_ip.to_string();
        
        // Create sorted list of attached routers (including DR itself)
        let mut router_list: Vec<String> = attached_routers.iter().cloned().collect();
        router_list.sort(); // Ensure consistent ordering
        
        // Calculate LSA length
        // Header: 20 bytes
        // Network mask: 4 bytes
        // Attached routers: 4 bytes each
        let length = 20 + 4 + (router_list.len() * 4) as u16;
        
        let header = LSAHeader {
            ls_age: 0,
            ls_type: LSAType::NetworkLSA,
            link_state_id,
            advertising_router: self.router_id.clone(),
            ls_sequence_number: sequence_number,
            ls_checksum: 0, // Will be calculated later
            length,
        };
        
        let network_lsa = NetworkLSA {
            network_mask: network_mask.to_string(),
            attached_routers: router_list,
        };
        
        console_log!(
            "Generated Network LSA: Link State ID={}, Mask={}, Attached Routers={:?}",
            header.link_state_id, network_mask, network_lsa.attached_routers
        );
        
        LSA {
            header,
            data: LSAData::Network(network_lsa),
        }
    }
    
    /// Check if a Network LSA needs to be generated or updated
    /// 
    /// A Network LSA should be (re)generated when:
    /// 1. The router becomes DR for the first time
    /// 2. A router becomes/ceases to be fully adjacent
    /// 3. The DR changes (old DR should flush its Network LSA)
    pub fn should_generate_network_lsa(
        &self,
        is_dr: bool,
        previous_attached_count: usize,
        current_attached_count: usize,
    ) -> bool {
        if !is_dr {
            return false;
        }
        
        // Generate if attached router count changed
        previous_attached_count != current_attached_count
    }
    
    /// Create a MaxAge Network LSA for flushing
    /// 
    /// When a router ceases to be DR, it should flush its Network LSA
    /// by setting LS Age to MaxAge (3600 seconds)
    pub fn create_maxage_network_lsa(
        &self,
        interface_ip: &str,
        network_mask: &str,
        sequence_number: u32,
    ) -> LSA {
        console_log!(
            "Router {} creating MaxAge Network LSA for interface {} (no longer DR)",
            self.router_id, interface_ip
        );
        
        let header = LSAHeader {
            ls_age: 3600, // MaxAge
            ls_type: LSAType::NetworkLSA,
            link_state_id: interface_ip.to_string(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: sequence_number,
            ls_checksum: 0,
            length: 24, // Minimum Network LSA size
        };
        
        let network_lsa = NetworkLSA {
            network_mask: network_mask.to_string(),
            attached_routers: Vec::new(), // Empty when flushing
        };
        
        LSA {
            header,
            data: LSAData::Network(network_lsa),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_network_lsa() {
        let generator = NetworkLSAGenerator::new("1.1.1.1".to_string());
        
        let mut attached_routers = HashSet::new();
        attached_routers.insert("1.1.1.1".to_string()); // DR itself
        attached_routers.insert("2.2.2.2".to_string());
        attached_routers.insert("3.3.3.3".to_string());
        
        let lsa = generator.generate_network_lsa(
            "192.168.1.1",
            "255.255.255.0",
            &attached_routers,
            0x80000001,
        );
        
        match &lsa.data {
            LSAData::Network(network_lsa) => {
                assert_eq!(network_lsa.network_mask, "255.255.255.0");
                assert_eq!(network_lsa.attached_routers.len(), 3);
                // Check sorted order
                assert_eq!(network_lsa.attached_routers[0], "1.1.1.1");
                assert_eq!(network_lsa.attached_routers[1], "2.2.2.2");
                assert_eq!(network_lsa.attached_routers[2], "3.3.3.3");
            }
            _ => panic!("Expected Network LSA"),
        }
        
        assert_eq!(lsa.header.link_state_id, "192.168.1.1");
        assert_eq!(lsa.header.advertising_router, "1.1.1.1");
        assert_eq!(lsa.header.ls_type, LSAType::NetworkLSA);
    }
    
    #[test]
    fn test_should_generate_network_lsa() {
        let generator = NetworkLSAGenerator::new("1.1.1.1".to_string());
        
        // Not DR - should not generate
        assert!(!generator.should_generate_network_lsa(false, 2, 3));
        
        // DR with no change - should not generate
        assert!(!generator.should_generate_network_lsa(true, 3, 3));
        
        // DR with attached router count change - should generate
        assert!(generator.should_generate_network_lsa(true, 2, 3));
        assert!(generator.should_generate_network_lsa(true, 3, 2));
    }
    
    #[test]
    fn test_create_maxage_network_lsa() {
        let generator = NetworkLSAGenerator::new("1.1.1.1".to_string());
        
        let lsa = generator.create_maxage_network_lsa(
            "192.168.1.1",
            "255.255.255.0",
            0x80000005,
        );
        
        assert_eq!(lsa.header.ls_age, 3600); // MaxAge
        assert_eq!(lsa.header.link_state_id, "192.168.1.1");
        
        match &lsa.data {
            LSAData::Network(network_lsa) => {
                assert_eq!(network_lsa.attached_routers.len(), 0);
            }
            _ => panic!("Expected Network LSA"),
        }
    }
}