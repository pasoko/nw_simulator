use serde::{Serialize, Deserialize};

/// OSPFv2 Options field implementation (RFC 2328 Section A.2)
/// 
/// The Options field is present in OSPF Hello packets, Database Description packets,
/// and all LSAs. It enables OSPF routers to communicate their optional capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OSPFOptions {
    /// The entire options field as a byte
    pub value: u8,
}

impl OSPFOptions {
    /// Create new OSPFOptions with default values
    pub fn new() -> Self {
        OSPFOptions { value: 0 }
    }

    /// Create OSPFOptions from a byte value
    pub fn from_byte(value: u8) -> Self {
        OSPFOptions { value }
    }

    /// Get the options as a byte value
    pub fn as_byte(&self) -> u8 {
        self.value
    }

    // RFC 2328 Section A.2 - Options field bits
    
    /// MT-bit (bit 0): Multi-Topology OSPF capability
    /// When set, indicates that the router supports Multi-Topology OSPF
    pub fn get_mt_bit(&self) -> bool {
        (self.value & 0x01) != 0
    }

    pub fn set_mt_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x01;
        } else {
            self.value &= !0x01;
        }
    }

    /// E-bit (bit 1): AS-External-LSA capability
    /// When set, indicates that the router is capable of receiving AS-External-LSAs
    /// This bit is set by default in all areas except stub areas
    pub fn get_e_bit(&self) -> bool {
        (self.value & 0x02) != 0
    }

    pub fn set_e_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x02;
        } else {
            self.value &= !0x02;
        }
    }

    /// MC-bit (bit 2): Multicast capability
    /// When set, indicates that the router supports IP multicast forwarding
    pub fn get_mc_bit(&self) -> bool {
        (self.value & 0x04) != 0
    }

    pub fn set_mc_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x04;
        } else {
            self.value &= !0x04;
        }
    }

    /// N/P-bit (bit 3): NSSA capability
    /// When set, indicates that the router supports NSSA (Not-So-Stubby-Area)
    pub fn get_np_bit(&self) -> bool {
        (self.value & 0x08) != 0
    }

    pub fn set_np_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x08;
        } else {
            self.value &= !0x08;
        }
    }

    /// L-bit (bit 4): LLS (Link Local Signaling) capability
    /// When set, indicates that the router supports LLS data block
    pub fn get_l_bit(&self) -> bool {
        (self.value & 0x10) != 0
    }

    pub fn set_l_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x10;
        } else {
            self.value &= !0x10;
        }
    }

    /// DC-bit (bit 5): Demand Circuit capability
    /// When set, indicates that the router supports demand circuits
    pub fn get_dc_bit(&self) -> bool {
        (self.value & 0x20) != 0
    }

    pub fn set_dc_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x20;
        } else {
            self.value &= !0x20;
        }
    }

    /// O-bit (bit 6): Opaque-LSA capability
    /// When set, indicates that the router supports Opaque-LSAs
    pub fn get_o_bit(&self) -> bool {
        (self.value & 0x40) != 0
    }

    pub fn set_o_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x40;
        } else {
            self.value &= !0x40;
        }
    }

    /// DN-bit (bit 7): Down bit
    /// Used in VPN applications to prevent loops
    pub fn get_dn_bit(&self) -> bool {
        (self.value & 0x80) != 0
    }

    pub fn set_dn_bit(&mut self, value: bool) {
        if value {
            self.value |= 0x80;
        } else {
            self.value &= !0x80;
        }
    }

    /// Create standard options for a normal area router
    pub fn standard_area_options() -> Self {
        let mut options = OSPFOptions::new();
        options.set_e_bit(true);  // Support AS-External-LSAs
        options.set_o_bit(true);  // Support Opaque-LSAs
        options
    }

    /// Create options for a stub area router
    pub fn stub_area_options() -> Self {
        let mut options = OSPFOptions::new();
        options.set_e_bit(false); // Do not support AS-External-LSAs in stub areas
        options.set_o_bit(true);  // Support Opaque-LSAs
        options
    }

    /// Create options for an NSSA area router
    pub fn nssa_area_options() -> Self {
        let mut options = OSPFOptions::new();
        options.set_e_bit(false); // Do not support AS-External-LSAs in NSSA
        options.set_np_bit(true); // Support NSSA capability
        options.set_o_bit(true);  // Support Opaque-LSAs
        options
    }

    /// Check if the options are compatible with another router's options
    pub fn is_compatible_with(&self, other: &OSPFOptions) -> bool {
        // E-bit must match for adjacency formation
        if self.get_e_bit() != other.get_e_bit() {
            return false;
        }
        
        // N/P-bit must match for NSSA areas
        if self.get_np_bit() != other.get_np_bit() {
            return false;
        }
        
        // Other bits are optional and don't affect compatibility
        true
    }

    /// Get a human-readable string representation of the options
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        
        if self.get_dn_bit() { parts.push("DN"); }
        if self.get_o_bit() { parts.push("O"); }
        if self.get_dc_bit() { parts.push("DC"); }
        if self.get_l_bit() { parts.push("L"); }
        if self.get_np_bit() { parts.push("N/P"); }
        if self.get_mc_bit() { parts.push("MC"); }
        if self.get_e_bit() { parts.push("E"); }
        if self.get_mt_bit() { parts.push("MT"); }
        
        if parts.is_empty() {
            "None".to_string()
        } else {
            parts.join(",")
        }
    }
}

impl Default for OSPFOptions {
    fn default() -> Self {
        OSPFOptions::standard_area_options()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_creation() {
        let options = OSPFOptions::new();
        assert_eq!(options.value, 0);
        assert!(!options.get_e_bit());
        assert!(!options.get_o_bit());
    }

    #[test]
    fn test_bit_manipulation() {
        let mut options = OSPFOptions::new();
        
        // Test E-bit
        options.set_e_bit(true);
        assert!(options.get_e_bit());
        assert_eq!(options.value, 0x02);
        
        options.set_e_bit(false);
        assert!(!options.get_e_bit());
        assert_eq!(options.value, 0x00);
        
        // Test multiple bits
        options.set_e_bit(true);
        options.set_o_bit(true);
        options.set_mc_bit(true);
        assert!(options.get_e_bit());
        assert!(options.get_o_bit());
        assert!(options.get_mc_bit());
        assert_eq!(options.value, 0x46); // 0x02 | 0x04 | 0x40
    }

    #[test]
    fn test_standard_area_options() {
        let options = OSPFOptions::standard_area_options();
        assert!(options.get_e_bit());
        assert!(options.get_o_bit());
        assert!(!options.get_np_bit());
    }

    #[test]
    fn test_stub_area_options() {
        let options = OSPFOptions::stub_area_options();
        assert!(!options.get_e_bit());
        assert!(options.get_o_bit());
        assert!(!options.get_np_bit());
    }

    #[test]
    fn test_nssa_area_options() {
        let options = OSPFOptions::nssa_area_options();
        assert!(!options.get_e_bit());
        assert!(options.get_np_bit());
        assert!(options.get_o_bit());
    }

    #[test]
    fn test_compatibility() {
        let standard1 = OSPFOptions::standard_area_options();
        let standard2 = OSPFOptions::standard_area_options();
        let stub = OSPFOptions::stub_area_options();
        let nssa = OSPFOptions::nssa_area_options();
        
        assert!(standard1.is_compatible_with(&standard2));
        assert!(!standard1.is_compatible_with(&stub));
        assert!(!standard1.is_compatible_with(&nssa));
        assert!(!stub.is_compatible_with(&nssa));
    }

    #[test]
    fn test_from_byte() {
        let options = OSPFOptions::from_byte(0x46); // E-bit, MC-bit, O-bit
        assert!(options.get_e_bit());
        assert!(options.get_mc_bit());
        assert!(options.get_o_bit());
        assert!(!options.get_np_bit());
    }

    #[test]
    fn test_to_string() {
        let mut options = OSPFOptions::new();
        assert_eq!(options.to_string(), "None");
        
        options.set_e_bit(true);
        options.set_o_bit(true);
        assert_eq!(options.to_string(), "O,E");
        
        options.set_mc_bit(true);
        options.set_np_bit(true);
        assert_eq!(options.to_string(), "O,N/P,MC,E");
    }
}