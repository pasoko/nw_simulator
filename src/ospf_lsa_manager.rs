use std::collections::HashMap;
use crate::router::{LSA, LSAData, LSAType, LSAHeader as RouterLSAHeader, RouterLSA, RouterLink, LinkType};
use crate::console_log;

/// OSPF LSA Database Management
/// 
/// Manages the Link State Advertisement database, including:
/// - LSA storage and retrieval
/// - LSA aging and expiration
/// - LSA generation
/// - Database synchronization
pub struct OSPFLSAManager {
    lsa_database: HashMap<String, LSA>,
    lsa_sequence_number: u32,
    router_id: String,
    router_links: Vec<(u32, u32, u32)>, // (neighbor_id, interface_id, cost)
}

impl OSPFLSAManager {
    pub fn new(router_id: String) -> Self {
        OSPFLSAManager {
            lsa_database: HashMap::new(),
            lsa_sequence_number: 0x80000001,
            router_id,
            router_links: Vec::new(),
        }
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        // Remove existing link to same neighbor first
        self.router_links.retain(|(n, _, _)| *n != neighbor_id);
        self.router_links.push((neighbor_id, interface_id, cost));
        console_log!("Router {} added link to neighbor {} on interface {} with cost {}", 
            self.router_id, neighbor_id, interface_id, cost);
    }
    
    pub fn remove_router_link(&mut self, neighbor_id: u32) {
        self.router_links.retain(|(n, _, _)| *n != neighbor_id);
        console_log!("Router {} removed link to neighbor {}", self.router_id, neighbor_id);
    }
    
    pub fn generate_router_lsa(&mut self) -> LSA {
        let mut links = Vec::new();
        
        // Add point-to-point links for ALL configured links
        for (neighbor_id, interface_id, cost) in &self.router_links {
            links.push(RouterLink {
                link_id: format!("1.1.1.{}", neighbor_id),
                link_data: format!("0.0.0.{}", interface_id),
                link_type: LinkType::PointToPoint,
                num_tos: 0,
                metric: *cost as u16,
            });
        }
        
        let router_lsa = RouterLSA {
            flags: 0x00,
            num_links: links.len() as u16,
            links,
        };
        
        let header = RouterLSAHeader {
            ls_age: 0,
            ls_type: LSAType::RouterLSA,
            link_state_id: self.router_id.clone(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: self.lsa_sequence_number,
            ls_checksum: 0,
            length: 20 + (router_lsa.links.len() * 12) as u16,
        };
        
        self.lsa_sequence_number += 1;
        
        let lsa = LSA {
            header,
            data: LSAData::Router(router_lsa.clone()),
        };
        
        // Add to database
        self.update_lsa_database(lsa.clone());
        console_log!("Router {} generated Router LSA with {} links", 
            self.router_id, router_lsa.num_links);
        
        lsa
    }
    
    pub fn update_lsa_database(&mut self, lsa: LSA) {
        let key = format!("{}:{}:{}", 
            lsa.header.ls_type.clone() as u8, 
            lsa.header.link_state_id.clone(), 
            lsa.header.advertising_router.clone()
        );
        
        console_log!("Router {} updating LSA database with key: {}", self.router_id, key);
        if let LSAData::Router(ref rlsa) = lsa.data {
            console_log!("  Router LSA with {} links", rlsa.links.len());
        }
        
        self.lsa_database.insert(key, lsa);
        console_log!("  LSA database now contains {} entries", self.lsa_database.len());
    }
    
    pub fn get_lsa_database(&self) -> &HashMap<String, LSA> {
        &self.lsa_database
    }
    
    pub fn get_lsa_count(&self) -> usize {
        self.lsa_database.len()
    }
    
    pub fn get_lsa_by_key(&self, key: &str) -> Option<&LSA> {
        self.lsa_database.get(key)
    }
    
    pub fn age_lsas(&mut self, time_delta: f64) {
        const MAX_AGE: u16 = 3600; // 1 hour in seconds
        let mut expired_lsas = Vec::new();
        
        for (key, lsa) in self.lsa_database.iter_mut() {
            let new_age = lsa.header.ls_age as f64 + time_delta;
            if new_age >= MAX_AGE as f64 {
                expired_lsas.push(key.clone());
            } else {
                lsa.header.ls_age = new_age as u16;
            }
        }
        
        // Remove expired LSAs
        for key in expired_lsas {
            self.lsa_database.remove(&key);
            console_log!("Router {} removed expired LSA: {}", self.router_id, key);
        }
    }
    
    pub fn find_lsa_to_request(&self, received_headers: &[RouterLSAHeader]) -> Vec<RouterLSAHeader> {
        let mut needed_lsas = Vec::new();
        
        for header in received_headers {
            let key = format!("{}:{}:{}", 
                header.ls_type.clone() as u8, 
                header.link_state_id, 
                header.advertising_router
            );
            
            let need_lsa = if let Some(our_lsa) = self.lsa_database.get(&key) {
                header.ls_sequence_number > our_lsa.header.ls_sequence_number
            } else {
                true
            };
            
            if need_lsa {
                needed_lsas.push(header.clone());
            }
        }
        
        needed_lsas
    }
    
    pub fn should_update_lsa(&self, new_lsa: &LSA) -> bool {
        let key = format!("{}:{}:{}", 
            new_lsa.header.ls_type.clone() as u8, 
            new_lsa.header.link_state_id, 
            new_lsa.header.advertising_router
        );
        
        if let Some(existing_lsa) = self.lsa_database.get(&key) {
            new_lsa.header.ls_sequence_number > existing_lsa.header.ls_sequence_number
        } else {
            true
        }
    }
    
    pub fn regenerate_router_lsa(&mut self) -> LSA {
        self.lsa_sequence_number += 1;
        console_log!("Router {} regenerating Router LSA due to topology change", self.router_id);
        self.generate_router_lsa()
    }
    
    pub fn clear_database(&mut self) {
        self.lsa_database.clear();
        console_log!("Router {} cleared LSA database", self.router_id);
    }
    
    pub fn get_router_links(&self) -> &Vec<(u32, u32, u32)> {
        &self.router_links
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsa_generation() {
        let mut manager = OSPFLSAManager::new("1.1.1.1".to_string());
        
        // Add links
        manager.add_router_link(2, 1, 10);
        manager.add_router_link(3, 2, 20);
        
        // Generate LSA
        let lsa = manager.generate_router_lsa();
        
        if let LSAData::Router(router_lsa) = &lsa.data {
            assert_eq!(router_lsa.num_links, 2);
            assert_eq!(router_lsa.links.len(), 2);
        } else {
            panic!("Expected Router LSA");
        }
        
        assert_eq!(manager.get_lsa_count(), 1);
    }
    
    #[test]
    fn test_lsa_aging() {
        let mut manager = OSPFLSAManager::new("1.1.1.1".to_string());
        let mut lsa = manager.generate_router_lsa();
        lsa.header.ls_age = 3500; // Close to expiration
        manager.update_lsa_database(lsa);
        
        // Age LSAs
        manager.age_lsas(200.0); // Should cause expiration
        
        assert_eq!(manager.get_lsa_count(), 0);
    }
}