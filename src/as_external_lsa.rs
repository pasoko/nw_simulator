use crate::router::{LSA, LSAType, LSAHeader, LSAData, ASExternalLSA};
use crate::console_log;

/// AS-External LSA (Type 5) Generator
/// 
/// Generates AS-External LSAs for routes from outside the OSPF domain.
/// ASBRs (Autonomous System Boundary Routers) use these to advertise external routes.
pub struct ASExternalLSAGenerator {
    router_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExternalMetricType {
    Type1 = 0, // Metric is added to internal cost
    Type2 = 1, // Metric is considered larger than any internal cost
}

impl ASExternalLSAGenerator {
    pub fn new(router_id: String) -> Self {
        ASExternalLSAGenerator {
            router_id,
        }
    }
    
    /// Generate an AS-External LSA for an external route
    /// 
    /// # Arguments
    /// * `network_address` - The external network address (e.g., "0.0.0.0" for default route)
    /// * `network_mask` - The network mask (e.g., "0.0.0.0" for default route)
    /// * `metric` - The external metric
    /// * `metric_type` - Type1 or Type2 metric
    /// * `forwarding_address` - Next hop for the external route (0.0.0.0 means use ASBR)
    /// * `external_route_tag` - Optional route tag for external routes
    /// * `sequence_number` - The LSA sequence number
    pub fn generate_as_external_lsa(
        &self,
        network_address: &str,
        network_mask: &str,
        metric: u32,
        metric_type: ExternalMetricType,
        forwarding_address: &str,
        external_route_tag: u32,
        sequence_number: u32,
    ) -> LSA {
        console_log!(
            "ASBR {} generating AS-External LSA for network {}/{} with metric {} (Type{})",
            self.router_id, network_address, network_mask, metric,
            if metric_type == ExternalMetricType::Type1 { "1" } else { "2" }
        );
        
        let external_data = ASExternalLSA {
            network_mask: network_mask.to_string(),
            metric: metric & 0x00FFFFFF, // 24-bit metric
            metric_type: metric_type as u8,
            forwarding_address: forwarding_address.to_string(),
            external_route_tag,
            tos: 0,
            tos_metric: 0,
        };
        
        let header = LSAHeader {
            ls_age: 0,
            ls_type: LSAType::ASExternalLSA,
            link_state_id: network_address.to_string(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: sequence_number,
            ls_checksum: 0, // Will be calculated later
            length: 20 + 20, // Header + AS-External data
        };
        
        let lsa = LSA {
            header,
            data: LSAData::ASExternal(external_data),
        };
        
        lsa
    }
    
    /// Generate a default route AS-External LSA
    /// 
    /// This is commonly used by ASBRs to inject a default route into OSPF
    pub fn generate_default_route_lsa(
        &self,
        metric: u32,
        metric_type: ExternalMetricType,
        sequence_number: u32,
    ) -> LSA {
        console_log!(
            "ASBR {} generating default route AS-External LSA",
            self.router_id
        );
        
        self.generate_as_external_lsa(
            "0.0.0.0",      // Default route
            "0.0.0.0",      // Mask for default route
            metric,
            metric_type,
            "0.0.0.0",      // Use ASBR as next hop
            0,              // No route tag
            sequence_number,
        )
    }
    
    /// Check if a router should generate AS-External LSAs
    /// 
    /// Returns true if the router is an ASBR (has external routes to advertise)
    pub fn should_generate_as_external_lsa(&self, has_external_routes: bool) -> bool {
        if has_external_routes {
            console_log!(
                "Router {} is ASBR and has external routes to advertise",
                self.router_id
            );
            true
        } else {
            false
        }
    }
    
    /// Generate AS-External LSAs for all external routes
    /// 
    /// # Arguments
    /// * `external_routes` - External routes to advertise
    /// * `sequence_number` - Starting sequence number
    pub fn generate_all_as_external_lsas(
        &self,
        external_routes: &[(String, String, u32, ExternalMetricType, String, u32)], // (network, mask, metric, type, fwd_addr, tag)
        mut sequence_number: u32,
    ) -> Vec<LSA> {
        let mut lsas = Vec::new();
        
        for (network, mask, metric, metric_type, fwd_addr, tag) in external_routes {
            console_log!(
                "ASBR {} generating AS-External LSA for external route {}/{}",
                self.router_id, network, mask
            );
            
            let lsa = self.generate_as_external_lsa(
                network,
                mask,
                *metric,
                *metric_type,
                fwd_addr,
                *tag,
                sequence_number,
            );
            
            lsas.push(lsa);
            sequence_number += 1;
        }
        
        console_log!(
            "ASBR {} generated {} AS-External LSAs",
            self.router_id, lsas.len()
        );
        
        lsas
    }
    
    /// Create a MaxAge AS-External LSA for flushing
    pub fn create_maxage_as_external_lsa(
        &self,
        network_address: &str,
        network_mask: &str,
        sequence_number: u32,
    ) -> LSA {
        let mut lsa = self.generate_as_external_lsa(
            network_address,
            network_mask,
            0xFFFFFF, // Max metric
            ExternalMetricType::Type2,
            "0.0.0.0",
            0,
            sequence_number,
        );
        
        lsa.header.ls_age = 3600; // MaxAge
        console_log!(
            "ASBR {} created MaxAge AS-External LSA for network {}/{}",
            self.router_id, network_address, network_mask
        );
        
        lsa
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_external_lsa_generation() {
        let generator = ASExternalLSAGenerator::new("1.1.1.1".to_string());
        
        let lsa = generator.generate_as_external_lsa(
            "10.0.0.0",
            "255.0.0.0",
            100,
            ExternalMetricType::Type2,
            "192.168.1.1",
            12345,
            1,
        );
        
        assert_eq!(lsa.header.ls_type, LSAType::ASExternalLSA);
        assert_eq!(lsa.header.link_state_id, "10.0.0.0");
        assert_eq!(lsa.header.advertising_router, "1.1.1.1");
        
        if let LSAData::ASExternal(external) = &lsa.data {
            assert_eq!(external.network_mask, "255.0.0.0");
            assert_eq!(external.metric, 100);
            assert_eq!(external.metric_type, 1); // Type2
            assert_eq!(external.forwarding_address, "192.168.1.1");
            assert_eq!(external.external_route_tag, 12345);
        } else {
            panic!("Expected AS-External LSA data");
        }
    }
    
    #[test]
    fn test_default_route_generation() {
        let generator = ASExternalLSAGenerator::new("1.1.1.1".to_string());
        
        let lsa = generator.generate_default_route_lsa(
            1,
            ExternalMetricType::Type1,
            1,
        );
        
        assert_eq!(lsa.header.link_state_id, "0.0.0.0");
        
        if let LSAData::ASExternal(external) = &lsa.data {
            assert_eq!(external.network_mask, "0.0.0.0");
            assert_eq!(external.metric, 1);
            assert_eq!(external.metric_type, 0); // Type1
            assert_eq!(external.forwarding_address, "0.0.0.0");
        } else {
            panic!("Expected AS-External LSA data");
        }
    }
    
    #[test]
    fn test_metric_type_values() {
        assert_eq!(ExternalMetricType::Type1 as u8, 0);
        assert_eq!(ExternalMetricType::Type2 as u8, 1);
    }
    
    #[test]
    fn test_multiple_external_routes() {
        let generator = ASExternalLSAGenerator::new("1.1.1.1".to_string());
        
        let routes = vec![
            ("10.0.0.0".to_string(), "255.0.0.0".to_string(), 100, ExternalMetricType::Type2, "0.0.0.0".to_string(), 0),
            ("172.16.0.0".to_string(), "255.240.0.0".to_string(), 200, ExternalMetricType::Type1, "192.168.1.1".to_string(), 100),
            ("0.0.0.0".to_string(), "0.0.0.0".to_string(), 1, ExternalMetricType::Type2, "0.0.0.0".to_string(), 0),
        ];
        
        let lsas = generator.generate_all_as_external_lsas(&routes, 1);
        
        assert_eq!(lsas.len(), 3);
        assert_eq!(lsas[0].header.link_state_id, "10.0.0.0");
        assert_eq!(lsas[1].header.link_state_id, "172.16.0.0");
        assert_eq!(lsas[2].header.link_state_id, "0.0.0.0"); // Default route
    }
}