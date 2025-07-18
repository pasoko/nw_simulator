use crate::router::{LSA, LSAType, LSAHeader, LSAData, SummaryLSA};
use crate::console_log;
use std::collections::HashSet;

/// Summary LSA (Type 3) Generator
/// 
/// Generates Summary LSAs for inter-area route advertisements.
/// ABRs use Summary LSAs to advertise destinations from one area into another.
pub struct SummaryLSAGenerator {
    router_id: String,
}

impl SummaryLSAGenerator {
    pub fn new(router_id: String) -> Self {
        SummaryLSAGenerator {
            router_id,
        }
    }
    
    /// Generate a Summary LSA for a network
    /// 
    /// # Arguments
    /// * `network_address` - The network address being advertised (e.g., "192.168.1.0")
    /// * `network_mask` - The network mask (e.g., "255.255.255.0")
    /// * `metric` - The cost to reach this network
    /// * `area_id` - The area into which this LSA is being advertised
    /// * `sequence_number` - The LSA sequence number
    pub fn generate_summary_lsa(
        &self,
        network_address: &str,
        network_mask: &str,
        metric: u32,
        area_id: &str,
        sequence_number: u32,
    ) -> LSA {
        console_log!(
            "ABR {} generating Summary LSA for network {}/{} into area {} with metric {}",
            self.router_id, network_address, network_mask, area_id, metric
        );
        
        let summary_data = SummaryLSA {
            network_mask: network_mask.to_string(),
            metric: metric & 0x00FFFFFF, // 24-bit metric
            tos: 0,
            tos_metric: 0,
        };
        
        let header = LSAHeader {
            ls_age: 0,
            ls_type: LSAType::SummaryLSA,
            link_state_id: network_address.to_string(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: sequence_number,
            ls_checksum: 0, // Will be calculated later
            length: 20 + 8, // Header + Summary data
        };
        
        let lsa = LSA {
            header,
            data: LSAData::Summary(summary_data),
        };
        
        lsa
    }
    
    /// Check if an ABR should generate Summary LSAs
    /// 
    /// Returns true if:
    /// - The router is an ABR (connected to multiple areas)
    /// - The router has routes to advertise from other areas
    pub fn should_generate_summary_lsa(
        &self,
        router_areas: &HashSet<String>,
        has_inter_area_routes: bool,
    ) -> bool {
        // Router must be connected to multiple areas to be an ABR
        let is_abr = router_areas.len() > 1;
        
        if is_abr && has_inter_area_routes {
            console_log!(
                "Router {} is ABR (connected to {} areas) and has inter-area routes",
                self.router_id, router_areas.len()
            );
            true
        } else {
            false
        }
    }
    
    /// Generate Summary LSAs for all inter-area routes
    /// 
    /// # Arguments
    /// * `inter_area_routes` - Routes learned from other areas
    /// * `target_area` - The area to advertise into
    /// * `sequence_number` - Starting sequence number
    pub fn generate_all_summary_lsas(
        &self,
        inter_area_routes: &[(String, String, u32, String)], // (network, mask, metric, source_area)
        target_area: &str,
        mut sequence_number: u32,
    ) -> Vec<LSA> {
        let mut lsas = Vec::new();
        
        for (network, mask, metric, source_area) in inter_area_routes {
            // Don't advertise routes back to their source area
            if source_area != target_area {
                console_log!(
                    "ABR {} generating Summary LSA for {}/{} from area {} into area {}",
                    self.router_id, network, mask, source_area, target_area
                );
                
                let lsa = self.generate_summary_lsa(
                    network,
                    mask,
                    *metric,
                    target_area,
                    sequence_number,
                );
                
                lsas.push(lsa);
                sequence_number += 1;
            }
        }
        
        console_log!(
            "ABR {} generated {} Summary LSAs for area {}",
            self.router_id, lsas.len(), target_area
        );
        
        lsas
    }
    
    /// Create a MaxAge Summary LSA for flushing
    pub fn create_maxage_summary_lsa(
        &self,
        network_address: &str,
        network_mask: &str,
        sequence_number: u32,
    ) -> LSA {
        let mut lsa = self.generate_summary_lsa(
            network_address,
            network_mask,
            0xFFFFFF, // Max metric indicates unreachable
            "0.0.0.0",
            sequence_number,
        );
        
        lsa.header.ls_age = 3600; // MaxAge
        console_log!(
            "ABR {} created MaxAge Summary LSA for network {}/{}",
            self.router_id, network_address, network_mask
        );
        
        lsa
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_lsa_generation() {
        let generator = SummaryLSAGenerator::new("1.1.1.1".to_string());
        
        let lsa = generator.generate_summary_lsa(
            "192.168.1.0",
            "255.255.255.0",
            10,
            "0.0.0.0",
            1,
        );
        
        assert_eq!(lsa.header.ls_type, LSAType::SummaryLSA);
        assert_eq!(lsa.header.link_state_id, "192.168.1.0");
        assert_eq!(lsa.header.advertising_router, "1.1.1.1");
        
        if let LSAData::Summary(summary) = &lsa.data {
            assert_eq!(summary.network_mask, "255.255.255.0");
            assert_eq!(summary.metric, 10);
        } else {
            panic!("Expected Summary LSA data");
        }
    }
    
    #[test]
    fn test_abr_detection() {
        let generator = SummaryLSAGenerator::new("1.1.1.1".to_string());
        
        // Single area - not an ABR
        let mut areas = HashSet::new();
        areas.insert("0.0.0.0".to_string());
        assert!(!generator.should_generate_summary_lsa(&areas, true));
        
        // Multiple areas - is an ABR
        areas.insert("0.0.0.1".to_string());
        assert!(generator.should_generate_summary_lsa(&areas, true));
        
        // Multiple areas but no routes - shouldn't generate
        assert!(!generator.should_generate_summary_lsa(&areas, false));
    }
    
    #[test]
    fn test_inter_area_route_filtering() {
        let generator = SummaryLSAGenerator::new("1.1.1.1".to_string());
        
        let routes = vec![
            ("192.168.1.0".to_string(), "255.255.255.0".to_string(), 10, "0.0.0.0".to_string()),
            ("192.168.2.0".to_string(), "255.255.255.0".to_string(), 20, "0.0.0.1".to_string()),
            ("192.168.3.0".to_string(), "255.255.255.0".to_string(), 30, "0.0.0.0".to_string()),
        ];
        
        // Advertise into area 0.0.0.0
        let lsas = generator.generate_all_summary_lsas(&routes, "0.0.0.0", 1);
        
        // Should only advertise route from area 0.0.0.1
        assert_eq!(lsas.len(), 1);
        assert_eq!(lsas[0].header.link_state_id, "192.168.2.0");
    }
}