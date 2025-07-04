use crate::router::{LSA, LSAData};
#[allow(unused_imports)]
use crate::console_log;

/// Fletcher's checksum implementation according to RFC 2328 Appendix B
/// This is used for OSPF LSA integrity verification
pub fn fletcher_checksum(data: &[u8]) -> u16 {
    // Special case for empty data
    if data.is_empty() {
        return 0xFFFF;
    }
    
    let mut c0: u32 = 0;
    let mut c1: u32 = 0;
    
    // Process the data in 16-bit chunks
    let chunks = data.chunks_exact(2);
    let remainder = chunks.remainder();
    
    // Process complete 16-bit words
    for chunk in chunks {
        let value = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        c0 = (c0 + value) % 255;
        c1 = (c1 + c0) % 255;
    }
    
    // Handle odd-length data
    if !remainder.is_empty() {
        let value = (remainder[0] as u32) << 8;
        c0 = (c0 + value) % 255;
        c1 = (c1 + c0) % 255;
    }
    
    // Calculate final checksum
    let x = ((255 - ((c0 + c1) % 255)) % 255) as u16;
    let y = ((255 - ((c0 + x as u32) % 255)) % 255) as u16;
    
    (x << 8) | y
}

/// Calculate checksum for an LSA according to RFC 2328
pub fn calculate_lsa_checksum(lsa: &LSA) -> u16 {
    let mut buffer = Vec::new();
    serialize_lsa_for_checksum(lsa, &mut buffer);
    let checksum = fletcher_checksum(&buffer);
    
    #[cfg(target_arch = "wasm32")]
    console_log!("[CHECKSUM] Calculated checksum {} for LSA type {:?}, ID: {}, AdvRouter: {}", 
        checksum, lsa.header.ls_type, lsa.header.link_state_id, lsa.header.advertising_router);
    
    checksum
}

/// Serialize LSA for checksum calculation (excluding age and checksum fields)
fn serialize_lsa_for_checksum(lsa: &LSA, buffer: &mut Vec<u8>) {
    // Skip LS age field (first 2 bytes)
    // Write LS type
    buffer.push(lsa.header.ls_type.clone() as u8);
    
    // Write Link State ID (4 bytes)
    for octet in lsa.header.link_state_id.split('.') {
        if let Ok(byte) = octet.parse::<u8>() {
            buffer.push(byte);
        }
    }
    
    // Write Advertising Router (4 bytes)
    for octet in lsa.header.advertising_router.split('.') {
        if let Ok(byte) = octet.parse::<u8>() {
            buffer.push(byte);
        }
    }
    
    // Write LS sequence number (4 bytes)
    buffer.extend_from_slice(&lsa.header.ls_sequence_number.to_be_bytes());
    
    // Skip checksum field (2 bytes)
    
    // Write length (2 bytes)
    buffer.extend_from_slice(&lsa.header.length.to_be_bytes());
    
    // Write LSA data
    match &lsa.data {
        LSAData::Router(router_lsa) => {
            buffer.push(router_lsa.flags);
            buffer.push(0); // Reserved
            buffer.extend_from_slice(&router_lsa.num_links.to_be_bytes());
            
            for link in &router_lsa.links {
                // Link ID (4 bytes)
                for octet in link.link_id.split('.') {
                    if let Ok(byte) = octet.parse::<u8>() {
                        buffer.push(byte);
                    }
                }
                
                // Link Data (4 bytes)
                for octet in link.link_data.split('.') {
                    if let Ok(byte) = octet.parse::<u8>() {
                        buffer.push(byte);
                    }
                }
                
                // Link Type
                buffer.push(link.link_type.clone() as u8);
                
                // Number of TOS
                buffer.push(link.num_tos);
                
                // Metric (2 bytes)
                buffer.extend_from_slice(&link.metric.to_be_bytes());
            }
        },
        _ => {
            // Other LSA types not implemented yet
        }
    }
}

/// Verify LSA checksum
pub fn verify_lsa_checksum(lsa: &LSA) -> bool {
    let calculated = calculate_lsa_checksum(lsa);
    let matches = calculated == lsa.header.ls_checksum;
    
    #[cfg(target_arch = "wasm32")]
    console_log!("[CHECKSUM] Verifying LSA checksum: calculated={}, stored={}, matches={}", 
        calculated, lsa.header.ls_checksum, matches);
    
    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::{LSAHeader, LSAType, RouterLSA};

    #[test]
    fn test_fletcher_checksum() {
        // Test with known data
        let data = b"Hello World";
        let checksum = fletcher_checksum(data);
        assert!(checksum != 0);
        
        // Test with empty data
        let empty_data = b"";
        let empty_checksum = fletcher_checksum(empty_data);
        assert_eq!(empty_checksum, 0xFFFF);
    }

    #[test]
    fn test_lsa_checksum() {
        let lsa = LSA {
            header: LSAHeader {
                ls_age: 0,
                ls_type: LSAType::RouterLSA,
                link_state_id: "1.1.1.1".to_string(),
                advertising_router: "1.1.1.1".to_string(),
                ls_sequence_number: 0x80000001,
                ls_checksum: 0,
                length: 20,
            },
            data: LSAData::Router(RouterLSA {
                flags: 0,
                num_links: 0,
                links: vec![],
            }),
        };
        
        let checksum = calculate_lsa_checksum(&lsa);
        assert!(checksum != 0);
    }
}