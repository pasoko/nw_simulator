use serde::{Deserialize, Serialize};
use crate::router::{LSA, LSAData, LSAHeader, LSAType};
use crate::console_log;

/// Opaque LSA Support for OSPFv2 (RFC 5250)
/// 
/// Opaque LSAs provide a mechanism to extend OSPF for new applications.
/// Three types are defined:
/// - Type 9: Link-local scope (not flooded beyond local link)
/// - Type 10: Area-local scope (flooded throughout single area)
/// - Type 11: AS-wide scope (flooded throughout AS)

/// Opaque LSA Type field structure
/// The Link State ID for Opaque LSAs is divided into:
/// - Opaque Type (8 bits): Application-specific type
/// - Opaque ID (24 bits): Type-specific ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpaqueLSAId {
    pub opaque_type: u8,
    pub opaque_id: u32, // Only 24 bits used
}

impl OpaqueLSAId {
    pub fn new(opaque_type: u8, opaque_id: u32) -> Self {
        // Ensure opaque_id fits in 24 bits
        let opaque_id = opaque_id & 0xFFFFFF;
        OpaqueLSAId {
            opaque_type,
            opaque_id,
        }
    }
    
    /// Convert to Link State ID string format
    pub fn to_link_state_id(&self) -> String {
        // Format: "Type.ID" where ID is the 24-bit value
        format!("{}.{}", self.opaque_type, self.opaque_id)
    }
    
    /// Parse from Link State ID string
    pub fn from_link_state_id(lsid: &str) -> Option<Self> {
        let parts: Vec<&str> = lsid.split('.').collect();
        if parts.len() == 2 {
            if let (Ok(opaque_type), Ok(opaque_id)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u32>()
            ) {
                return Some(OpaqueLSAId::new(opaque_type, opaque_id));
            }
        }
        None
    }
}

/// Standard Opaque Types (RFC 5250 and extensions)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardOpaqueType {
    TrafficEngineering = 1,      // RFC 3630
    SRAlgorithm = 4,             // RFC 8665 (Segment Routing)
    ExtendedPrefix = 7,          // RFC 7684
    ExtendedLink = 8,            // RFC 7684
    ApplicationSpecific = 255,    // For custom applications
}

/// Opaque LSA Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaqueLSAData {
    /// Application-specific data
    pub data: Vec<u8>,
}

/// Traffic Engineering LSA (RFC 3630)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficEngineeringLSA {
    pub router_address: String,
    pub links: Vec<TELink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TELink {
    pub link_type: u8,
    pub link_id: String,
    pub local_interface_ip: String,
    pub remote_interface_ip: String,
    pub metric: u32,
    pub max_bandwidth: f32,
    pub max_reservable_bandwidth: f32,
    pub unreserved_bandwidth: [f32; 8], // 8 priority levels
    pub admin_group: u32,
}

/// Opaque LSA Generator
pub struct OpaqueLSAGenerator {
    router_id: String,
    sequence_number: u32,
}

impl OpaqueLSAGenerator {
    pub fn new(router_id: String) -> Self {
        OpaqueLSAGenerator {
            router_id,
            sequence_number: 0x80000001,
        }
    }
    
    /// Generate a Type 9 Opaque LSA (Link-local scope)
    pub fn generate_type9_opaque_lsa(
        &mut self,
        _interface_id: u32,
        opaque_type: u8,
        opaque_id: u32,
        data: Vec<u8>,
    ) -> LSA {
        let lsa_id = OpaqueLSAId::new(opaque_type, opaque_id);
        self.generate_opaque_lsa(LSAType::OpaqueLinkLocal, lsa_id, data)
    }
    
    /// Generate a Type 10 Opaque LSA (Area-local scope)
    pub fn generate_type10_opaque_lsa(
        &mut self,
        opaque_type: u8,
        opaque_id: u32,
        data: Vec<u8>,
    ) -> LSA {
        let lsa_id = OpaqueLSAId::new(opaque_type, opaque_id);
        self.generate_opaque_lsa(LSAType::OpaqueAreaLocal, lsa_id, data)
    }
    
    /// Generate a Type 11 Opaque LSA (AS-wide scope)
    pub fn generate_type11_opaque_lsa(
        &mut self,
        opaque_type: u8,
        opaque_id: u32,
        data: Vec<u8>,
    ) -> LSA {
        let lsa_id = OpaqueLSAId::new(opaque_type, opaque_id);
        self.generate_opaque_lsa(LSAType::OpaqueASWide, lsa_id, data)
    }
    
    /// Generate a Traffic Engineering LSA (Type 10, Opaque Type 1)
    pub fn generate_te_lsa(
        &mut self,
        router_address: String,
        te_links: Vec<TELink>,
    ) -> LSA {
        // Serialize TE data
        let te_lsa = TrafficEngineeringLSA {
            router_address,
            links: te_links,
        };
        
        let data = serde_json::to_vec(&te_lsa).unwrap_or_default();
        
        // TE LSAs use Opaque Type 1
        self.generate_type10_opaque_lsa(
            StandardOpaqueType::TrafficEngineering as u8,
            1, // Opaque ID
            data,
        )
    }
    
    fn generate_opaque_lsa(
        &mut self,
        lsa_type: LSAType,
        lsa_id: OpaqueLSAId,
        data: Vec<u8>,
    ) -> LSA {
        let header = LSAHeader {
            ls_age: 0,
            ls_type: lsa_type,
            link_state_id: lsa_id.to_link_state_id(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: self.sequence_number,
            ls_checksum: 0, // Will be calculated
            length: 20 + data.len() as u16,
        };
        
        self.sequence_number = self.increment_sequence_number();
        
        let data_len = data.len();
        let opaque_data = OpaqueLSAData { data };
        
        let mut lsa = LSA {
            header,
            data: LSAData::Opaque(opaque_data),
        };
        
        // Calculate checksum
        lsa.header.ls_checksum = crate::ospf_checksum::calculate_lsa_checksum(&lsa);
        
        console_log!(
            "Generated {} Opaque LSA: type={}, id={}, size={} bytes",
            match lsa_type {
                LSAType::OpaqueLinkLocal => "Type 9",
                LSAType::OpaqueAreaLocal => "Type 10", 
                LSAType::OpaqueASWide => "Type 11",
                _ => "Unknown",
            },
            lsa_id.opaque_type,
            lsa_id.opaque_id,
            data_len
        );
        
        lsa
    }
    
    fn increment_sequence_number(&mut self) -> u32 {
        let current = self.sequence_number;
        if current == 0x7FFFFFFF {
            self.sequence_number = 0x80000001;
        } else {
            self.sequence_number = current + 1;
        }
        current
    }
}

/// Opaque LSA Processor
pub struct OpaqueLSAProcessor;

impl OpaqueLSAProcessor {
    /// Process received Opaque LSA
    pub fn process_opaque_lsa(lsa: &LSA) -> Result<(), String> {
        match &lsa.data {
            LSAData::Opaque(opaque_data) => {
                // Parse the Opaque LSA ID
                if let Some(opaque_id) = OpaqueLSAId::from_link_state_id(&lsa.header.link_state_id) {
                    console_log!(
                        "Processing Opaque LSA: type={}, opaque_type={}, opaque_id={}, size={}",
                        lsa.header.ls_type as u8,
                        opaque_id.opaque_type,
                        opaque_id.opaque_id,
                        opaque_data.data.len()
                    );
                    
                    // Handle specific opaque types
                    match opaque_id.opaque_type {
                        1 => {
                            // Traffic Engineering LSA
                            if let Ok(te_lsa) = serde_json::from_slice::<TrafficEngineeringLSA>(&opaque_data.data) {
                                console_log!(
                                    "Traffic Engineering LSA: router={}, links={}",
                                    te_lsa.router_address,
                                    te_lsa.links.len()
                                );
                            }
                        }
                        _ => {
                            console_log!(
                                "Unknown Opaque Type {}, treating as opaque data",
                                opaque_id.opaque_type
                            );
                        }
                    }
                    
                    Ok(())
                } else {
                    Err("Invalid Opaque LSA ID format".to_string())
                }
            }
            _ => Err("Not an Opaque LSA".to_string()),
        }
    }
    
    /// Check if router supports Opaque LSAs
    pub fn is_opaque_capable(options: &crate::ospf_options::OSPFOptions) -> bool {
        options.get_o_bit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_opaque_lsa_id() {
        let id = OpaqueLSAId::new(1, 0x123456);
        assert_eq!(id.opaque_type, 1);
        assert_eq!(id.opaque_id, 0x123456);
        
        let lsid = id.to_link_state_id();
        assert_eq!(lsid, "1.1193046");
        
        let parsed = OpaqueLSAId::from_link_state_id(&lsid).unwrap();
        assert_eq!(parsed.opaque_type, id.opaque_type);
        assert_eq!(parsed.opaque_id, id.opaque_id);
    }
    
    #[test]
    fn test_opaque_lsa_generation() {
        let mut generator = OpaqueLSAGenerator::new("1.1.1.1".to_string());
        
        // Generate Type 9 LSA
        let data = vec![1, 2, 3, 4, 5];
        let lsa = generator.generate_type9_opaque_lsa(1, 1, 100, data.clone());
        
        assert_eq!(lsa.header.ls_type, LSAType::OpaqueLinkLocal);
        assert_eq!(lsa.header.link_state_id, "1.100");
        
        if let LSAData::Opaque(opaque) = &lsa.data {
            assert_eq!(opaque.data, data);
        } else {
            panic!("Expected Opaque LSA data");
        }
    }
    
    #[test]
    fn test_te_lsa_generation() {
        let mut generator = OpaqueLSAGenerator::new("1.1.1.1".to_string());
        
        let te_link = TELink {
            link_type: 1,
            link_id: "1.1.1.2".to_string(),
            local_interface_ip: "10.0.0.1".to_string(),
            remote_interface_ip: "10.0.0.2".to_string(),
            metric: 10,
            max_bandwidth: 1000.0,
            max_reservable_bandwidth: 800.0,
            unreserved_bandwidth: [800.0; 8],
            admin_group: 0,
        };
        
        let lsa = generator.generate_te_lsa(
            "1.1.1.1".to_string(),
            vec![te_link],
        );
        
        assert_eq!(lsa.header.ls_type, LSAType::OpaqueAreaLocal);
        
        // Verify it's a TE LSA (Opaque Type 1)
        let opaque_id = OpaqueLSAId::from_link_state_id(&lsa.header.link_state_id).unwrap();
        assert_eq!(opaque_id.opaque_type, StandardOpaqueType::TrafficEngineering as u8);
    }
}