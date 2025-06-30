use serde::{Serialize, Deserialize};

/// OSPF Network Type (RFC 2328 Section 1.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OSPFNetworkType {
    /// Point-to-Point network
    /// - No DR/BDR election
    /// - Only two routers on the network
    PointToPoint,
    
    /// Broadcast network (e.g., Ethernet)
    /// - DR/BDR election required
    /// - All routers can communicate directly
    Broadcast,
    
    /// Non-Broadcast Multi-Access (NBMA) network
    /// - DR/BDR election required
    /// - Neighbors must be configured manually
    NBMA,
    
    /// Point-to-Multipoint network
    /// - No DR/BDR election
    /// - Treated as collection of point-to-point links
    PointToMultipoint,
}

impl Default for OSPFNetworkType {
    fn default() -> Self {
        // Default to Point-to-Multipoint for current behavior
        OSPFNetworkType::PointToMultipoint
    }
}

impl OSPFNetworkType {
    /// Check if this network type requires DR/BDR election
    pub fn requires_dr_election(&self) -> bool {
        match self {
            OSPFNetworkType::Broadcast | OSPFNetworkType::NBMA => true,
            OSPFNetworkType::PointToPoint | OSPFNetworkType::PointToMultipoint => false,
        }
    }
    
    /// Get the default network mask for this network type
    pub fn default_network_mask(&self) -> &'static str {
        match self {
            OSPFNetworkType::PointToPoint => "255.255.255.252", // /30
            OSPFNetworkType::Broadcast => "255.255.255.0",      // /24
            OSPFNetworkType::NBMA => "255.255.255.0",          // /24
            OSPFNetworkType::PointToMultipoint => "255.255.255.0", // /24
        }
    }
    
    /// Check if Hello packets should be sent to all neighbors
    pub fn send_hello_to_all(&self) -> bool {
        match self {
            OSPFNetworkType::PointToMultipoint => true,
            OSPFNetworkType::PointToPoint => true,
            OSPFNetworkType::Broadcast => true,
            OSPFNetworkType::NBMA => false, // Unicast only
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_type_properties() {
        assert!(OSPFNetworkType::Broadcast.requires_dr_election());
        assert!(OSPFNetworkType::NBMA.requires_dr_election());
        assert!(!OSPFNetworkType::PointToPoint.requires_dr_election());
        assert!(!OSPFNetworkType::PointToMultipoint.requires_dr_election());
    }
    
    #[test]
    fn test_default_masks() {
        assert_eq!(OSPFNetworkType::PointToPoint.default_network_mask(), "255.255.255.252");
        assert_eq!(OSPFNetworkType::Broadcast.default_network_mask(), "255.255.255.0");
    }
}