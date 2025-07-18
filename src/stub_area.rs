use std::collections::HashSet;
use crate::router::{LSA, LSAType};
use crate::ospf_options::OSPFOptions;
use crate::console_log;

/// Stub Area Support for OSPFv2 (RFC 2328 Section 3.6)
/// 
/// Stub areas are areas through which or into which AS-External-LSAs are not flooded.
/// All routers in a stub area must be configured with the E-bit set to 0.
/// 
/// Key characteristics:
/// - No Type 5 LSAs (AS-External) allowed
/// - Default route injected by ABR as Type 3 Summary LSA
/// - All routers must agree on stub status (E-bit = 0)
/// - Cannot be transit area for virtual links
/// - Cannot contain an ASBR

#[derive(Debug, Clone, PartialEq)]
pub enum AreaType {
    /// Normal area - allows all LSA types
    Normal,
    /// Stub area - blocks Type 5 LSAs
    Stub {
        /// Cost of the default route injected by ABR
        default_cost: u32,
        /// Whether to suppress Type 3 LSAs (totally stubby)
        no_summary: bool,
    },
    /// Not-So-Stubby Area (NSSA) - allows Type 7 LSAs
    NSSA {
        /// Whether to inject default route
        default_originate: bool,
        /// Cost of the default route if injected
        default_cost: u32,
        /// Whether to suppress Type 3 LSAs (totally NSSA)
        no_summary: bool,
    },
}

impl AreaType {
    /// Create a standard stub area
    pub fn stub(default_cost: u32) -> Self {
        AreaType::Stub {
            default_cost,
            no_summary: false,
        }
    }
    
    /// Create a totally stubby area (Cisco proprietary)
    pub fn totally_stub(default_cost: u32) -> Self {
        AreaType::Stub {
            default_cost,
            no_summary: true,
        }
    }
    
    /// Create a standard NSSA
    pub fn nssa(default_originate: bool, default_cost: u32) -> Self {
        AreaType::NSSA {
            default_originate,
            default_cost,
            no_summary: false,
        }
    }
    
    /// Create a totally NSSA (Cisco proprietary)
    pub fn totally_nssa(default_originate: bool, default_cost: u32) -> Self {
        AreaType::NSSA {
            default_originate,
            default_cost,
            no_summary: true,
        }
    }
    
    /// Check if area allows AS-External LSAs (Type 5)
    pub fn allows_external_lsas(&self) -> bool {
        matches!(self, AreaType::Normal)
    }
    
    /// Check if area allows Type 7 LSAs (NSSA External)
    pub fn allows_nssa_lsas(&self) -> bool {
        matches!(self, AreaType::NSSA { .. })
    }
    
    /// Check if area suppresses inter-area routes (Type 3)
    pub fn suppresses_summary_lsas(&self) -> bool {
        match self {
            AreaType::Stub { no_summary, .. } => *no_summary,
            AreaType::NSSA { no_summary, .. } => *no_summary,
            AreaType::Normal => false,
        }
    }
    
    /// Get the appropriate OSPF options for this area type
    pub fn get_ospf_options(&self) -> OSPFOptions {
        match self {
            AreaType::Normal => OSPFOptions::standard_area_options(),
            AreaType::Stub { .. } => OSPFOptions::stub_area_options(),
            AreaType::NSSA { .. } => OSPFOptions::nssa_area_options(),
        }
    }
    
    /// Get default route cost for stub/NSSA areas
    pub fn get_default_cost(&self) -> Option<u32> {
        match self {
            AreaType::Stub { default_cost, .. } => Some(*default_cost),
            AreaType::NSSA { default_originate: true, default_cost, .. } => Some(*default_cost),
            _ => None,
        }
    }
}

/// Stub Area Manager
/// 
/// Manages stub area configuration and LSA filtering
pub struct StubAreaManager {
    /// Area ID
    area_id: String,
    /// Area type configuration
    area_type: AreaType,
    /// Whether this router is an ABR
    is_abr: bool,
    /// Connected areas (for ABR functionality)
    connected_areas: HashSet<String>,
}

impl StubAreaManager {
    pub fn new(area_id: String, area_type: AreaType) -> Self {
        StubAreaManager {
            area_id,
            area_type,
            is_abr: false,
            connected_areas: HashSet::new(),
        }
    }
    
    /// Update ABR status based on connected areas
    pub fn update_abr_status(&mut self, connected_areas: HashSet<String>) {
        self.connected_areas = connected_areas;
        // Router is ABR if connected to multiple areas including backbone (0.0.0.0)
        self.is_abr = self.connected_areas.len() > 1 && 
                      self.connected_areas.contains("0.0.0.0");
        
        if self.is_abr {
            console_log!("Router is now ABR for area {}", self.area_id);
        }
    }
    
    /// Filter LSA based on area type
    pub fn should_accept_lsa(&self, lsa: &LSA) -> bool {
        match lsa.header.ls_type {
            LSAType::ASExternalLSA => {
                // Type 5 LSAs only allowed in normal areas
                if !self.area_type.allows_external_lsas() {
                    console_log!("Blocking Type 5 LSA in stub area {}", self.area_id);
                    return false;
                }
            }
            LSAType::SummaryLSA | LSAType::SummaryASBR => {
                // Type 3/4 LSAs may be blocked in totally stub/NSSA areas
                if self.area_type.suppresses_summary_lsas() && !self.is_abr {
                    console_log!("Blocking Type 3/4 LSA in totally stub area {}", self.area_id);
                    return false;
                }
            }
            _ => {
                // Other LSA types are always allowed
            }
        }
        true
    }
    
    /// Generate default route LSA for stub area (ABR only)
    pub fn generate_default_route_lsa(&self, router_id: String) -> Option<LSA> {
        if !self.is_abr {
            return None;
        }
        
        if let Some(cost) = self.area_type.get_default_cost() {
            // Create Type 3 Summary LSA for default route (0.0.0.0)
            let header = crate::router::LSAHeader {
                ls_age: 0,
                ls_type: LSAType::SummaryLSA,
                link_state_id: "0.0.0.0".to_string(),
                advertising_router: router_id.clone(),
                ls_sequence_number: 0x80000001,
                ls_checksum: 0, // Will be calculated
                length: 28, // Summary LSA header + body
            };
            
            let data = crate::router::LSAData::Summary(crate::router::SummaryLSA {
                network_mask: "0.0.0.0".to_string(),
                metric: cost,
                tos: 0,
                tos_metric: 0,
            });
            
            let mut lsa = LSA { header, data };
            
            // Calculate checksum
            lsa.header.ls_checksum = crate::ospf_checksum::calculate_lsa_checksum(&lsa);
            
            console_log!(
                "ABR {} generated default route LSA for stub area {} with cost {}",
                router_id, self.area_id, cost
            );
            
            Some(lsa)
        } else {
            None
        }
    }
    
    /// Check if area configuration is valid
    pub fn validate_configuration(&self) -> Result<(), String> {
        // Stub areas cannot be transit for virtual links
        if matches!(self.area_type, AreaType::Stub { .. }) {
            // This would be checked when configuring virtual links
            // For now, just return OK
        }
        
        // Area 0 (backbone) cannot be stub
        if self.area_id == "0.0.0.0" && !matches!(self.area_type, AreaType::Normal) {
            return Err("Backbone area (0.0.0.0) cannot be configured as stub".to_string());
        }
        
        Ok(())
    }
    
    /// Get area type
    pub fn get_area_type(&self) -> &AreaType {
        &self.area_type
    }
    
    /// Set area type
    pub fn set_area_type(&mut self, area_type: AreaType) -> Result<(), String> {
        self.area_type = area_type;
        self.validate_configuration()
    }
    
    /// Check if router is ABR
    pub fn is_abr(&self) -> bool {
        self.is_abr
    }
    
    /// Get OSPF options for this area
    pub fn get_ospf_options(&self) -> OSPFOptions {
        self.area_type.get_ospf_options()
    }
}

/// Type 7 to Type 5 LSA Translation (for NSSA ABR)
pub struct NSSATranslator {
    /// Whether this ABR is performing translation
    performing_translation: bool,
    /// Translated LSAs (Type 7 ID -> Type 5 LSA)
    translations: HashMap<String, LSA>,
}

impl NSSATranslator {
    pub fn new() -> Self {
        NSSATranslator {
            performing_translation: false,
            translations: HashMap::new(),
        }
    }
    
    /// Translate Type 7 LSA to Type 5 LSA
    pub fn translate_type7_to_type5(&mut self, type7_lsa: &LSA, translator_id: String) -> Option<LSA> {
        if type7_lsa.header.ls_type != LSAType::OpaqueASWide {
            return None;
        }
        
        // Create Type 5 LSA from Type 7
        let mut type5_lsa = type7_lsa.clone();
        type5_lsa.header.ls_type = LSAType::ASExternalLSA;
        type5_lsa.header.advertising_router = translator_id;
        type5_lsa.header.ls_sequence_number = 0x80000001;
        
        // Recalculate checksum
        type5_lsa.header.ls_checksum = crate::ospf_checksum::calculate_lsa_checksum(&type5_lsa);
        
        let key = format!("{}:{}", type7_lsa.header.link_state_id, type7_lsa.header.advertising_router);
        self.translations.insert(key, type5_lsa.clone());
        
        console_log!(
            "Translated Type 7 LSA {} to Type 5 LSA",
            type7_lsa.header.link_state_id
        );
        
        Some(type5_lsa)
    }
    
    /// Start performing translation
    pub fn start_translation(&mut self) {
        self.performing_translation = true;
        console_log!("NSSA ABR started Type 7 to Type 5 translation");
    }
    
    /// Stop performing translation
    pub fn stop_translation(&mut self) {
        self.performing_translation = false;
        self.translations.clear();
        console_log!("NSSA ABR stopped Type 7 to Type 5 translation");
    }
    
    /// Check if performing translation
    pub fn is_translating(&self) -> bool {
        self.performing_translation
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_area_types() {
        let normal = AreaType::Normal;
        assert!(normal.allows_external_lsas());
        assert!(!normal.allows_nssa_lsas());
        assert!(!normal.suppresses_summary_lsas());
        
        let stub = AreaType::stub(10);
        assert!(!stub.allows_external_lsas());
        assert!(!stub.allows_nssa_lsas());
        assert!(!stub.suppresses_summary_lsas());
        assert_eq!(stub.get_default_cost(), Some(10));
        
        let totally_stub = AreaType::totally_stub(20);
        assert!(!totally_stub.allows_external_lsas());
        assert!(totally_stub.suppresses_summary_lsas());
        
        let nssa = AreaType::nssa(true, 30);
        assert!(!nssa.allows_external_lsas());
        assert!(nssa.allows_nssa_lsas());
        assert_eq!(nssa.get_default_cost(), Some(30));
    }
    
    #[test]
    fn test_stub_area_manager() {
        let mut manager = StubAreaManager::new(
            "1.0.0.0".to_string(),
            AreaType::stub(10)
        );
        
        // Not ABR initially
        assert!(!manager.is_abr());
        
        // Update ABR status
        let mut areas = HashSet::new();
        areas.insert("0.0.0.0".to_string());
        areas.insert("1.0.0.0".to_string());
        manager.update_abr_status(areas);
        assert!(manager.is_abr());
        
        // Test LSA filtering
        let type5_lsa = LSA {
            header: crate::router::LSAHeader {
                ls_age: 0,
                ls_type: LSAType::ASExternalLSA,
                link_state_id: "10.0.0.0".to_string(),
                advertising_router: "1.1.1.1".to_string(),
                ls_sequence_number: 1,
                ls_checksum: 0,
                length: 36,
            },
            data: crate::router::LSAData::ASExternal(crate::router::ASExternalLSA {
                network_mask: "255.255.255.0".to_string(),
                metric: 10,
                metric_type: 1,
                forwarding_address: "0.0.0.0".to_string(),
                external_route_tag: 0,
                tos: 0,
                tos_metric: 0,
            }),
        };
        
        // Stub area should block Type 5 LSAs
        assert!(!manager.should_accept_lsa(&type5_lsa));
    }
    
    #[test]
    fn test_default_route_generation() {
        let mut manager = StubAreaManager::new(
            "1.0.0.0".to_string(),
            AreaType::stub(10)
        );
        
        // Not ABR, should not generate default route
        assert!(manager.generate_default_route_lsa("1.1.1.1".to_string()).is_none());
        
        // Make it ABR
        let mut areas = HashSet::new();
        areas.insert("0.0.0.0".to_string());
        areas.insert("1.0.0.0".to_string());
        manager.update_abr_status(areas);
        
        // Now should generate default route
        let default_lsa = manager.generate_default_route_lsa("1.1.1.1".to_string());
        assert!(default_lsa.is_some());
        
        let lsa = default_lsa.unwrap();
        assert_eq!(lsa.header.ls_type, LSAType::SummaryLSA);
        assert_eq!(lsa.header.link_state_id, "0.0.0.0");
        
        if let crate::router::LSAData::Summary(summary) = &lsa.data {
            assert_eq!(summary.metric, 10);
        } else {
            panic!("Expected Summary LSA data");
        }
    }
}