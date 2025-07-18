use serde::{Serialize, Deserialize};
use std::convert::TryFrom;

/// OSPFv2 Authentication Types (RFC 2328 Section D)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AuthType {
    /// No authentication
    Null = 0,
    /// Simple password authentication
    SimplePassword = 1,
    /// Cryptographic authentication (MD5)
    CryptographicMD5 = 2,
}

impl TryFrom<u16> for AuthType {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AuthType::Null),
            1 => Ok(AuthType::SimplePassword),
            2 => Ok(AuthType::CryptographicMD5),
            _ => Err(format!("Invalid authentication type: {}", value)),
        }
    }
}

/// Authentication data for OSPF packets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthData {
    /// No authentication data
    None,
    /// Null authentication (same as None, but more explicit)
    Null,
    /// Simple password (8 bytes)
    SimplePassword([u8; 8]),
    /// Cryptographic authentication
    Cryptographic {
        key_id: u8,
        auth_data_len: u8,
        crypto_sequence_number: u32,
        message_digest: Vec<u8>,
    },
}

impl AuthData {
    /// Create authentication data from packet fields
    pub fn from_packet(auth_type: AuthType, auth_field: u64) -> Self {
        match auth_type {
            AuthType::Null => AuthData::Null,
            AuthType::SimplePassword => {
                let bytes = auth_field.to_be_bytes();
                AuthData::SimplePassword(bytes)
            }
            AuthType::CryptographicMD5 => {
                // For MD5, the auth_field contains key_id and auth_data_len
                // The actual digest is appended to the packet
                AuthData::Cryptographic {
                    key_id: (auth_field >> 56) as u8,
                    auth_data_len: ((auth_field >> 48) & 0xFF) as u8,
                    crypto_sequence_number: (auth_field & 0xFFFFFFFF) as u32,
                    message_digest: Vec::new(), // To be filled from packet appendix
                }
            }
        }
    }

    /// Convert to packet field representation
    pub fn to_packet_field(&self) -> u64 {
        match self {
            AuthData::None | AuthData::Null => 0,
            AuthData::SimplePassword(password) => {
                u64::from_be_bytes(*password)
            }
            AuthData::Cryptographic { key_id, auth_data_len, crypto_sequence_number, .. } => {
                ((*key_id as u64) << 56) |
                ((*auth_data_len as u64) << 48) |
                (*crypto_sequence_number as u64)
            }
        }
    }
}

/// Authentication configuration for an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub auth_key: Option<String>,
    pub key_id: Option<u8>,
    pub crypto_sequence_number: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            auth_type: AuthType::Null,
            auth_key: None,
            key_id: None,
            crypto_sequence_number: 0,
        }
    }
}

impl AuthConfig {
    /// Create a new authentication configuration
    pub fn new(auth_type: AuthType) -> Self {
        AuthConfig {
            auth_type,
            ..Default::default()
        }
    }

    /// Set simple password authentication
    pub fn with_simple_password(mut self, password: String) -> Self {
        self.auth_type = AuthType::SimplePassword;
        self.auth_key = Some(password);
        self
    }

    /// Set MD5 cryptographic authentication
    pub fn with_md5(mut self, key: String, key_id: u8) -> Self {
        self.auth_type = AuthType::CryptographicMD5;
        self.auth_key = Some(key);
        self.key_id = Some(key_id);
        self
    }

    /// Generate authentication data for a packet
    pub fn generate_auth_data(&mut self) -> (AuthType, AuthData) {
        match self.auth_type {
            AuthType::Null => (AuthType::Null, AuthData::Null),
            AuthType::SimplePassword => {
                if let Some(password) = &self.auth_key {
                    let mut bytes = [0u8; 8];
                    let password_bytes = password.as_bytes();
                    let len = password_bytes.len().min(8);
                    bytes[..len].copy_from_slice(&password_bytes[..len]);
                    (AuthType::SimplePassword, AuthData::SimplePassword(bytes))
                } else {
                    (AuthType::Null, AuthData::Null)
                }
            }
            AuthType::CryptographicMD5 => {
                // Increment sequence number for each packet
                self.crypto_sequence_number = self.crypto_sequence_number.wrapping_add(1);
                
                let auth_data = AuthData::Cryptographic {
                    key_id: self.key_id.unwrap_or(1),
                    auth_data_len: 16, // MD5 digest length
                    crypto_sequence_number: self.crypto_sequence_number,
                    message_digest: Vec::new(), // To be calculated
                };
                (AuthType::CryptographicMD5, auth_data)
            }
        }
    }
}

/// Verify authentication of a received packet
pub fn verify_authentication(
    auth_type: AuthType,
    auth_data: &AuthData,
    expected_config: &AuthConfig,
) -> Result<(), String> {
    match (auth_type, auth_data, &expected_config.auth_type) {
        (AuthType::Null, AuthData::None | AuthData::Null, AuthType::Null) => Ok(()),
        
        (AuthType::SimplePassword, AuthData::SimplePassword(received), AuthType::SimplePassword) => {
            if let Some(expected_password) = &expected_config.auth_key {
                let mut expected_bytes = [0u8; 8];
                let password_bytes = expected_password.as_bytes();
                let len = password_bytes.len().min(8);
                expected_bytes[..len].copy_from_slice(&password_bytes[..len]);
                
                if received == &expected_bytes {
                    Ok(())
                } else {
                    Err("Simple password authentication failed".to_string())
                }
            } else {
                Err("No password configured for simple authentication".to_string())
            }
        }
        
        (AuthType::CryptographicMD5, AuthData::Cryptographic { .. }, AuthType::CryptographicMD5) => {
            // MD5 verification would be implemented here
            // For now, we'll accept it if the types match
            Ok(())
        }
        
        _ => Err(format!(
            "Authentication type mismatch: received {:?}, expected {:?}",
            auth_type, expected_config.auth_type
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_type_conversion() {
        assert_eq!(AuthType::try_from(0).unwrap(), AuthType::Null);
        assert_eq!(AuthType::try_from(1).unwrap(), AuthType::SimplePassword);
        assert_eq!(AuthType::try_from(2).unwrap(), AuthType::CryptographicMD5);
        assert!(AuthType::try_from(3).is_err());
    }

    #[test]
    fn test_simple_password_auth() {
        let mut config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("secret".to_string());
        
        let (auth_type, auth_data) = config.generate_auth_data();
        
        assert_eq!(auth_type, AuthType::SimplePassword);
        match auth_data {
            AuthData::SimplePassword(bytes) => {
                assert_eq!(&bytes[..6], b"secret");
                assert_eq!(bytes[6], 0);
                assert_eq!(bytes[7], 0);
            }
            _ => panic!("Expected SimplePassword auth data"),
        }
    }

    #[test]
    fn test_auth_verification() {
        let config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("test1234".to_string());
        
        let mut bytes = [0u8; 8];
        bytes[..8].copy_from_slice(b"test1234");
        let auth_data = AuthData::SimplePassword(bytes);
        
        assert!(verify_authentication(
            AuthType::SimplePassword,
            &auth_data,
            &config
        ).is_ok());
        
        // Test with wrong password
        let wrong_config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("wrong".to_string());
        
        assert!(verify_authentication(
            AuthType::SimplePassword,
            &auth_data,
            &wrong_config
        ).is_err());
    }
}