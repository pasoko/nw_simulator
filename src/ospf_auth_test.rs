#[cfg(test)]
mod tests {
    use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
    use crate::ospf_auth::{AuthType, AuthData, AuthConfig, verify_authentication};
    use crate::console_log;

    #[test]
    fn test_ospf_packet_with_null_auth() {
        let packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: "1.1.1.1".to_string(),
            area_id: "0.0.0.0".to_string(),
            checksum: 0,
            auth_type: AuthType::Null,
            auth_data: AuthData::None,
            data: OSPFPacketData::Hello(HelloPacket {
                network_mask: "255.255.255.0".to_string(),
                hello_interval: 10,
                options: 0x02,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: vec![],
            }),
        };

        let config = AuthConfig::default();
        
        // 認証検証が成功することを確認
        assert!(verify_authentication(packet.auth_type, &packet.auth_data, &config).is_ok());
    }

    #[test]
    fn test_ospf_packet_with_simple_password() {
        let mut config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("testpass".to_string());
        
        let auth_data = config.generate_auth_data();
        
        let packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: "2.2.2.2".to_string(),
            area_id: "0.0.0.0".to_string(),
            checksum: 0,
            auth_type: AuthType::SimplePassword,
            auth_data: auth_data.clone(),
            data: OSPFPacketData::Hello(HelloPacket {
                network_mask: "255.255.255.0".to_string(),
                hello_interval: 10,
                options: 0x02,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: vec!["1.1.1.1".to_string()],
            }),
        };

        // 同じパスワードで検証成功
        assert!(verify_authentication(packet.auth_type, &packet.auth_data, &config).is_ok());
        
        // 異なるパスワードで検証失敗
        let wrong_config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("wrongpass".to_string());
        assert!(verify_authentication(packet.auth_type, &packet.auth_data, &wrong_config).is_err());
    }

    #[test]
    fn test_ospf_packet_auth_type_mismatch() {
        // Null認証のパケット
        let packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: "3.3.3.3".to_string(),
            area_id: "0.0.0.0".to_string(),
            checksum: 0,
            auth_type: AuthType::Null,
            auth_data: AuthData::None,
            data: OSPFPacketData::Hello(HelloPacket {
                network_mask: "255.255.255.0".to_string(),
                hello_interval: 10,
                options: 0x02,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: vec![],
            }),
        };

        // SimplePassword認証を期待する設定
        let config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("password".to_string());
        
        // 認証タイプの不一致で失敗
        assert!(verify_authentication(packet.auth_type, &packet.auth_data, &config).is_err());
    }

    #[test]
    fn test_ospf_packet_serialization_with_auth() {
        let mut config = AuthConfig::new(AuthType::SimplePassword)
            .with_simple_password("serialize".to_string());
        
        let auth_data = config.generate_auth_data();
        
        let packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: "4.4.4.4".to_string(),
            area_id: "0.0.0.0".to_string(),
            checksum: 0x1234,
            auth_type: AuthType::SimplePassword,
            auth_data,
            data: OSPFPacketData::Hello(HelloPacket {
                network_mask: "255.255.255.0".to_string(),
                hello_interval: 10,
                options: 0x02,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "1.1.1.1".to_string(),
                backup_designated_router: "2.2.2.2".to_string(),
                neighbors: vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()],
            }),
        };

        // シリアライズとデシリアライズができることを確認
        let serialized = serde_json::to_string(&packet).unwrap();
        console_log!("Serialized packet: {}", serialized);
        
        let deserialized: OSPFPacket = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.auth_type, AuthType::SimplePassword);
        assert_eq!(deserialized.router_id, "4.4.4.4");
        
        // デシリアライズされたパケットの認証も検証できることを確認
        assert!(verify_authentication(deserialized.auth_type, &deserialized.auth_data, &config).is_ok());
    }

    #[test]
    fn test_md5_auth_data_creation() {
        let mut config = AuthConfig::new(AuthType::CryptographicMD5)
            .with_md5("md5secret".to_string(), 1);
        
        let auth_data = config.generate_auth_data();
        
        match auth_data {
            AuthData::Cryptographic { key_id, auth_data_len, crypto_sequence_number, .. } => {
                assert_eq!(key_id, 1);
                assert_eq!(auth_data_len, 16); // MD5 digest length
                assert_eq!(crypto_sequence_number, 1); // First sequence number
                
                // 2回目の生成でシーケンス番号が増加することを確認
                let auth_data2 = config.generate_auth_data();
                match auth_data2 {
                    AuthData::Cryptographic { crypto_sequence_number: seq2, .. } => {
                        assert_eq!(seq2, 2);
                    }
                    _ => panic!("Expected Cryptographic auth data"),
                }
            }
            _ => panic!("Expected Cryptographic auth data"),
        }
    }
}