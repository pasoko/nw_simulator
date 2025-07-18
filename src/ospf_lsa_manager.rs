use std::collections::{HashMap, HashSet};
use crate::router::{LSA, LSAData, LSAType, LSAHeader as RouterLSAHeader, RouterLSA, RouterLink, LinkType};
use crate::ospf_checksum::calculate_lsa_checksum;
use crate::ospf_lsa_age_manager::{LSAAgeManager, MAX_AGE, MIN_LS_INTERVAL, INITIAL_SEQUENCE_NUMBER, MAX_SEQUENCE_NUMBER};
use crate::console_log;

// Constants are now imported from ospf_lsa_age_manager

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
    recent_lsa_updates: HashMap<String, f64>, // Track recent LSA updates to prevent flooding loops
    current_time: f64, // Track current simulation time
    maxage_lsas_pending_purge: HashMap<String, f64>, // Track MaxAge LSAs pending deletion
    database_updated: bool, // Track if database was updated since last check
    age_manager: LSAAgeManager, // Enhanced age management
}

impl OSPFLSAManager {
    pub fn new(router_id: String) -> Self {
        OSPFLSAManager {
            lsa_database: HashMap::new(),
            lsa_sequence_number: INITIAL_SEQUENCE_NUMBER,
            router_id,
            router_links: Vec::new(),
            recent_lsa_updates: HashMap::new(),
            current_time: 0.0,
            maxage_lsas_pending_purge: HashMap::new(),
            database_updated: false,
            age_manager: LSAAgeManager::new(),
        }
    }
    
    pub fn add_router_link(&mut self, neighbor_id: u32, interface_id: u32, cost: u32) {
        // Remove existing link to same neighbor first
        self.router_links.retain(|(n, _, _)| *n != neighbor_id);
        self.router_links.push((neighbor_id, interface_id, cost));
        console_log!("Router {} added link to neighbor {} on interface {} with cost {}", 
            self.router_id, neighbor_id, interface_id, cost);
    }
    
    pub fn get_next_sequence_number(&mut self) -> u32 {
        let seq = self.lsa_sequence_number;
        self.lsa_sequence_number = if seq >= MAX_SEQUENCE_NUMBER {
            INITIAL_SEQUENCE_NUMBER
        } else {
            seq + 1
        };
        seq
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
        
        // Increment sequence number BEFORE using it
        self.lsa_sequence_number = self.increment_sequence_number();
        
        let header = RouterLSAHeader {
            ls_age: 0,
            ls_type: LSAType::RouterLSA,
            link_state_id: self.router_id.clone(),
            advertising_router: self.router_id.clone(),
            ls_sequence_number: self.lsa_sequence_number,
            ls_checksum: 0,
            length: 20 + (router_lsa.links.len() * 12) as u16,
        };
        
        let mut lsa = LSA {
            header,
            data: LSAData::Router(router_lsa.clone()),
        };
        
        // Calculate checksum
        lsa.header.ls_checksum = calculate_lsa_checksum(&lsa);
        
        // Add to database
        self.update_lsa_database(lsa.clone());
        console_log!("Router {} generated Router LSA with {} links, seq num {}, checksum {}", 
            self.router_id, router_lsa.num_links, self.lsa_sequence_number, lsa.header.ls_checksum);
        
        lsa
    }
    
    pub fn add_lsa(&mut self, lsa: LSA) -> bool {
        self.update_lsa_database(lsa);
        true
    }
    
    pub fn update_lsa_database(&mut self, lsa: LSA) {
        self.update_lsa_database_with_interface(lsa, None);
    }
    
    pub fn update_lsa_database_with_interface(&mut self, lsa: LSA, interface_id: Option<u32>) {
        console_log!("Router {} update_lsa_database called for LSA from {}", 
            self.router_id, lsa.header.advertising_router);
        let key = format!("{}:{}:{}", 
            lsa.header.ls_type.clone() as u8, 
            lsa.header.link_state_id.clone(), 
            lsa.header.advertising_router.clone()
        );
        
        // Do NOT track update time here - it should only be tracked after successful flooding
        // This prevents MinLSInterval from blocking the initial flood of a newly generated LSA
        
        console_log!("Router {} updating LSA database with key: {}", self.router_id, key);
        if let LSAData::Router(ref rlsa) = lsa.data {
            console_log!("  Router LSA with {} links", rlsa.links.len());
        }
        
        // Record LSA in age manager
        if lsa.header.advertising_router == self.router_id {
            self.age_manager.record_self_originated_lsa(key.clone(), &lsa);
        } else {
            self.age_manager.record_received_lsa(key.clone(), &lsa, interface_id);
        }
        
        self.lsa_database.insert(key.clone(), lsa);
        self.database_updated = true; // Mark that database was updated
        console_log!("  LSA database now contains {} entries, database_updated set to true for key: {}", 
            self.lsa_database.len(), key);
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
    
    pub fn age_lsas(&mut self, _time_delta: f64) -> Vec<LSA> {
        let mut maxage_lsas = Vec::new();
        
        // Update age manager time
        self.age_manager.update_time(self.current_time);
        
        for (key, lsa) in self.lsa_database.iter_mut() {
            // Skip if already at MaxAge
            if lsa.header.ls_age == MAX_AGE {
                continue;
            }
            
            // Use age manager to calculate age
            let new_age = self.age_manager.calculate_age(key);
            
            if new_age >= MAX_AGE {
                // Set to MaxAge but don't remove yet - need to reflood first
                lsa.header.ls_age = MAX_AGE;
                maxage_lsas.push(lsa.clone());
                self.maxage_lsas_pending_purge.insert(key.clone(), self.current_time);
                console_log!("Router {} LSA {} reached MaxAge, marking for reflooding", self.router_id, key);
            } else {
                lsa.header.ls_age = new_age;
            }
        }
        
        // Check for LSAs needing refresh
        let refresh_needed = self.age_manager.get_lsas_needing_refresh();
        for lsa_key in refresh_needed {
            console_log!("Router {} LSA {} needs refresh", self.router_id, lsa_key);
            // Actual refresh will be handled by the OSPF engine
        }
        
        // OSPFv2 RFC 2328: MaxAge LSAs should be kept in the database
        // They are only removed when:
        // 1. A newer version of the LSA is received
        // 2. The originating router is no longer reachable (determined by SPF calculation)
        // For now, we'll keep MaxAge LSAs indefinitely to comply with OSPFv2
        // The proper implementation would check reachability after SPF calculation
        
        // Log MaxAge LSAs but don't remove them
        for (key, reflood_time) in &self.maxage_lsas_pending_purge {
            if self.current_time - reflood_time > 60.0 {
                console_log!("Router {} keeping MaxAge LSA in database (OSPFv2 compliance): {}", 
                    self.router_id, key);
            }
        }
        
        // Clean up old entries from recent updates tracking
        self.recent_lsa_updates.retain(|_, &mut update_time| {
            self.age_manager.check_min_ls_interval("", update_time)
        });
        
        maxage_lsas
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
            // RFC 2328 Section 13.1: LSA is more recent if:
            // 1. It has a higher sequence number
            // 2. Same sequence but higher checksum 
            // 3. Same sequence and checksum but age is MaxAge while current is not
            if new_lsa.header.ls_sequence_number > existing_lsa.header.ls_sequence_number {
                console_log!("  LSA {} has newer seq num {} > {}, will update", 
                    key, new_lsa.header.ls_sequence_number, existing_lsa.header.ls_sequence_number);
                return true;
            } else if new_lsa.header.ls_sequence_number < existing_lsa.header.ls_sequence_number {
                console_log!("  LSA {} has older seq num {} < {}, skipping update", 
                    key, new_lsa.header.ls_sequence_number, existing_lsa.header.ls_sequence_number);
                return false;
            }
            
            // Same sequence number - check if it's truly the same LSA
            if new_lsa.header.ls_checksum == existing_lsa.header.ls_checksum {
                console_log!("  LSA {} already exists with same seq num {} and checksum, skipping update", 
                    key, new_lsa.header.ls_sequence_number);
                return false;
            }
            
            // Different checksum with same sequence is unusual but update anyway
            console_log!("  LSA {} has same seq num {} but different checksum, will update", 
                key, new_lsa.header.ls_sequence_number);
            true
        } else {
            console_log!("  LSA {} is new, will update", key);
            true
        }
    }
    
    pub fn regenerate_router_lsa(&mut self) -> LSA {
        // Check if we really need to regenerate (topology changed)
        if self.needs_lsa_regeneration() {
            console_log!("Router {} regenerating Router LSA due to topology change", self.router_id);
            let lsa = self.generate_router_lsa();
            
            // Mark as refreshed in age manager
            let key = format!("1:{}:{}", self.router_id, self.router_id);
            self.age_manager.mark_refreshed(&key, lsa.header.ls_sequence_number);
            
            // Mark database as updated when we generate a new LSA
            self.database_updated = true;
            lsa
        } else {
            // Return existing LSA without incrementing sequence number
            console_log!("Router {} LSA regeneration requested but topology unchanged", self.router_id);
            let key = format!("1:{}:{}", self.router_id, self.router_id);
            self.lsa_database.get(&key).cloned().unwrap_or_else(|| {
                console_log!("Router {} has no existing LSA, generating new one", self.router_id);
                let lsa = self.generate_router_lsa();
                
                // Mark as refreshed in age manager
                self.age_manager.mark_refreshed(&key, lsa.header.ls_sequence_number);
                
                // Mark database as updated when we generate a new LSA
                self.database_updated = true;
                lsa
            })
        }
    }
    
    pub fn needs_lsa_regeneration(&self) -> bool {
        // Check if current LSA matches our configured links
        let key = format!("1:{}:{}", self.router_id, self.router_id);
        if let Some(existing_lsa) = self.lsa_database.get(&key) {
            if let LSAData::Router(router_lsa) = &existing_lsa.data {
                // Check if number of links matches
                if router_lsa.links.len() != self.router_links.len() {
                    console_log!("Router {} link count changed: {} -> {}", 
                        self.router_id, router_lsa.links.len(), self.router_links.len());
                    return true;
                }
                
                // Check if all current links are present in the LSA
                for (neighbor_id, interface_id, cost) in &self.router_links {
                    let link_found = router_lsa.links.iter().any(|link| {
                        link.link_id == format!("1.1.1.{}", neighbor_id) &&
                        link.link_data == format!("0.0.0.{}", interface_id) &&
                        link.metric == *cost as u16
                    });
                    
                    if !link_found {
                        console_log!("Router {} link to neighbor {} not found in current LSA, regeneration needed", 
                            self.router_id, neighbor_id);
                        return true;
                    }
                }
                
                // Check if LSA contains links that are no longer configured
                for link in &router_lsa.links {
                    if let Some(neighbor_id_str) = link.link_id.split('.').last() {
                        if let Ok(neighbor_id) = neighbor_id_str.parse::<u32>() {
                            let link_exists = self.router_links.iter().any(|(n, _, _)| *n == neighbor_id);
                            if !link_exists {
                                console_log!("Router {} LSA contains link to neighbor {} which is no longer configured, regeneration needed", 
                                    self.router_id, neighbor_id);
                                return true;
                            }
                        }
                    }
                }
                
                return false;
            }
        }
        // No existing LSA, so we need to generate one
        true
    }
    
    pub fn clear_database(&mut self) {
        self.lsa_database.clear();
        console_log!("Router {} cleared LSA database", self.router_id);
    }
    
    pub fn get_router_links(&self) -> &Vec<(u32, u32, u32)> {
        &self.router_links
    }
    
    pub fn get_lsa_by_header(&self, header: &crate::ospf::LSAHeader) -> Option<&LSA> {
        let key = format!("{}:{}:{}", 
            header.lsa_type, 
            header.link_state_id, 
            header.advertising_router
        );
        self.lsa_database.get(&key)
    }
    
    pub fn was_recently_updated(&self, lsa_key: &str, current_time: f64) -> bool {
        if let Some(&update_time) = self.recent_lsa_updates.get(lsa_key) {
            current_time - update_time < MIN_LS_INTERVAL as f64
        } else {
            false
        }
    }
    
    pub fn mark_lsa_flooded(&mut self, lsa_key: &str) {
        // Mark this LSA as recently flooded to prevent flooding loops
        self.recent_lsa_updates.insert(lsa_key.to_string(), self.current_time);
        console_log!("Router {} marked LSA {} as flooded at time {:.2}", 
            self.router_id, lsa_key, self.current_time);
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
        self.age_manager.update_time(time);
    }
    
    fn increment_sequence_number(&mut self) -> u32 {
        if let Some(new_seq) = LSAAgeManager::increment_sequence_number(self.lsa_sequence_number) {
            new_seq
        } else {
            // Sequence number wrap - need to flush LSA
            console_log!("Router {} sequence number wrapped from {} to {}", 
                self.router_id, MAX_SEQUENCE_NUMBER, INITIAL_SEQUENCE_NUMBER);
            INITIAL_SEQUENCE_NUMBER
        }
    }
    
    pub fn get_maxage_lsas(&self) -> Vec<LSA> {
        self.lsa_database.values()
            .filter(|lsa| lsa.header.ls_age == MAX_AGE)
            .cloned()
            .collect()
    }
    
    pub fn was_database_updated(&self) -> bool {
        self.database_updated
    }
    
    pub fn reset_database_updated(&mut self) {
        self.database_updated = false;
    }
    
    /// Remove LSAs from unreachable routers after SPF calculation
    /// This should be called after SPF calculation with the set of reachable router IDs
    pub fn remove_unreachable_lsas(&mut self, reachable_routers: &HashSet<u32>) {
        let mut lsas_to_remove = Vec::new();
        
        for (key, lsa) in &self.lsa_database {
            // Extract router ID from advertising router field (format: "1.1.1.X")
            if let Some(router_id) = lsa.header.advertising_router
                .split('.')
                .last()
                .and_then(|s| s.parse::<u32>().ok()) {
                
                // Only remove LSAs from unreachable routers if they are MaxAge
                if lsa.header.ls_age == MAX_AGE && !reachable_routers.contains(&router_id) {
                    lsas_to_remove.push(key.clone());
                    console_log!("Router {} will remove MaxAge LSA from unreachable router {}: {}", 
                        self.router_id, router_id, key);
                }
            }
        }
        
        for key in lsas_to_remove {
            self.lsa_database.remove(&key);
            self.recent_lsa_updates.remove(&key);
            self.maxage_lsas_pending_purge.remove(&key);
            self.age_manager.remove_lsa(&key);
            console_log!("Router {} removed LSA from unreachable router: {}", self.router_id, key);
        }
    }
    
    /// Get all LSAs with their current ages
    pub fn get_all_lsa_ages(&mut self) -> HashMap<String, u16> {
        self.age_manager.get_all_lsa_ages()
    }
    
    /// Set InfTransDelay for an interface
    pub fn set_interface_delay(&mut self, interface_id: u32, delay: u16) {
        self.age_manager.set_interface_delay(interface_id, delay);
        console_log!("Router {} set InfTransDelay {} for interface {}", 
            self.router_id, delay, interface_id);
    }
    
    /// Get InfTransDelay for an interface
    pub fn get_interface_delay(&self, interface_id: u32) -> u16 {
        self.age_manager.get_interface_delay(interface_id)
    }
    
    /// Check if an LSA needs refresh
    pub fn needs_lsa_refresh(&self, lsa_key: &str) -> bool {
        self.age_manager.needs_refresh(lsa_key)
    }
    
    /// Get LSAs that need refresh
    pub fn get_lsas_needing_refresh(&self) -> Vec<String> {
        self.age_manager.get_lsas_needing_refresh()
    }
    
    /// Get age manager for testing
    #[cfg(test)]
    pub fn get_age_manager_mut(&mut self) -> &mut LSAAgeManager {
        &mut self.age_manager
    }
    
    /// Get age manager
    pub fn get_age_manager(&self) -> &LSAAgeManager {
        &self.age_manager
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
        manager.update_lsa_database(lsa.clone());
        
        // Age LSAs to MaxAge
        let maxage_lsas = manager.age_lsas(200.0); // Should reach MaxAge
        
        // Should return LSAs that reached MaxAge
        assert_eq!(maxage_lsas.len(), 1);
        assert_eq!(maxage_lsas[0].header.ls_age, MAX_AGE);
        
        // LSA should still be in database (pending purge)
        assert_eq!(manager.get_lsa_count(), 1);
        
        // Advance time past grace period
        manager.update_time(61.0);
        manager.age_lsas(0.0); // Trigger cleanup
        
        // OSPFv2 compliance: MaxAge LSAs are kept in database
        // They are only removed when a newer version is received or
        // the originating router is no longer reachable
        assert_eq!(manager.get_lsa_count(), 1);
    }
}