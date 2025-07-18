use std::collections::HashMap;
use crate::router::{RouterInterface, OSPFNeighborState};
use crate::ospf_options::OSPFOptions;
use crate::console_log;

/// Virtual Link Support for OSPFv2 (RFC 2328 Section 15)
/// 
/// Virtual links are used to connect physically separate parts of the backbone
/// through a non-backbone area. They are also used to connect areas that do not
/// have a direct physical connection to the backbone.
/// 
/// Key characteristics:
/// - Must be configured between two ABRs
/// - Transit area cannot be a stub area
/// - Treated as unnumbered point-to-point links
/// - Use router IDs as endpoints
/// - Inherit most parameters from transit area

#[derive(Debug, Clone)]
pub struct VirtualLink {
    /// Local router ID (ABR)
    pub local_router_id: String,
    
    /// Remote router ID (ABR)
    pub remote_router_id: String,
    
    /// Transit area ID (cannot be 0.0.0.0 or stub)
    pub transit_area_id: String,
    
    /// Virtual link state
    pub state: VirtualLinkState,
    
    /// Cost of the virtual link (sum of costs through transit area)
    pub cost: u32,
    
    /// Hello interval (inherited from transit area)
    pub hello_interval: u16,
    
    /// Dead interval (inherited from transit area)
    pub dead_interval: u32,
    
    /// Retransmit interval
    pub rxmt_interval: u16,
    
    /// Authentication configuration
    pub auth_type: u16,
    pub auth_key: Vec<u8>,
    
    /// Neighbor state for the virtual link
    pub neighbor_state: OSPFNeighborState,
    
    /// Options field for the virtual link
    pub options: OSPFOptions,
    
    /// Last time hello was received
    pub last_hello_time: Option<f64>,
    
    /// Virtual interface ID
    pub interface_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VirtualLinkState {
    /// Virtual link is down
    Down,
    /// Virtual link is point-to-point (operational)
    PointToPoint,
    /// Waiting for neighbor
    Waiting,
}

impl VirtualLink {
    pub fn new(
        local_router_id: String,
        remote_router_id: String,
        transit_area_id: String,
        interface_id: u32,
    ) -> Self {
        VirtualLink {
            local_router_id,
            remote_router_id,
            transit_area_id,
            state: VirtualLinkState::Down,
            cost: 0,
            hello_interval: 10,
            dead_interval: 40,
            rxmt_interval: 5,
            auth_type: 0,
            auth_key: vec![],
            neighbor_state: OSPFNeighborState::Down,
            options: OSPFOptions::standard_area_options(),
            last_hello_time: None,
            interface_id,
        }
    }
    
    /// Check if virtual link can become operational
    pub fn can_activate(&self, local_is_abr: bool, remote_is_abr: bool, path_exists: bool) -> bool {
        // Both endpoints must be ABRs
        if !local_is_abr || !remote_is_abr {
            return false;
        }
        
        // Path must exist through transit area
        if !path_exists {
            return false;
        }
        
        // Transit area cannot be backbone
        if self.transit_area_id == "0.0.0.0" {
            return false;
        }
        
        true
    }
    
    /// Update virtual link state
    pub fn update_state(&mut self, new_state: VirtualLinkState) {
        if self.state != new_state {
            console_log!(
                "Virtual link {} -> {} state changed: {:?} -> {:?}",
                self.local_router_id, self.remote_router_id,
                self.state, new_state
            );
            self.state = new_state;
        }
    }
    
    /// Update cost based on shortest path through transit area
    pub fn update_cost(&mut self, new_cost: u32) {
        if self.cost != new_cost {
            console_log!(
                "Virtual link {} -> {} cost updated: {} -> {}",
                self.local_router_id, self.remote_router_id,
                self.cost, new_cost
            );
            self.cost = new_cost;
        }
    }
    
    /// Create a virtual interface representation
    pub fn to_interface(&self) -> RouterInterface {
        RouterInterface {
            id: self.interface_id,
            name: format!("VL-{}-{}", self.local_router_id, self.remote_router_id),
            ip_address: "0.0.0.0".to_string(), // Unnumbered
            netmask: "255.255.255.255".to_string(),
            cost: self.cost,
            hello_interval: self.hello_interval,
            dead_interval: self.dead_interval as u16,
            priority: 0, // Not applicable for P2P
            connected_router_id: if self.neighbor_state != OSPFNeighborState::Down {
                // Parse remote router ID to u32 if possible
                self.remote_router_id.split('.').collect::<Vec<_>>()[0].parse::<u32>().ok()
            } else {
                None
            },
            enabled: self.state == VirtualLinkState::PointToPoint,
            mtu: 1500,
            inf_trans_delay: 1, // 1 second default
            rxmt_interval: self.rxmt_interval,
            manual_config: true,
            auth_config: crate::ospf_auth::AuthConfig::default(),
        }
    }
}

/// Virtual Link Manager
/// 
/// Manages all virtual links for a router
pub struct VirtualLinkManager {
    /// Configured virtual links (keyed by remote router ID)
    virtual_links: HashMap<String, VirtualLink>,
    
    /// Local router ID
    local_router_id: String,
    
    /// Current time
    current_time: f64,
    
    /// Next available interface ID for virtual links
    next_interface_id: u32,
}

impl VirtualLinkManager {
    pub fn new(local_router_id: String, starting_interface_id: u32) -> Self {
        VirtualLinkManager {
            virtual_links: HashMap::new(),
            local_router_id,
            current_time: 0.0,
            next_interface_id: starting_interface_id,
        }
    }
    
    /// Update current time
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
        
        // Check for dead virtual links
        let mut dead_links = Vec::new();
        for (remote_id, vlink) in &self.virtual_links {
            if let Some(last_hello) = vlink.last_hello_time {
                if self.current_time - last_hello > vlink.dead_interval as f64 {
                    dead_links.push(remote_id.clone());
                }
            }
        }
        
        // Mark dead links as down
        for remote_id in dead_links {
            if let Some(vlink) = self.virtual_links.get_mut(&remote_id) {
                vlink.update_state(VirtualLinkState::Down);
                vlink.neighbor_state = OSPFNeighborState::Down;
                console_log!(
                    "Virtual link {} -> {} timed out",
                    self.local_router_id, remote_id
                );
            }
        }
    }
    
    /// Configure a new virtual link
    pub fn configure_virtual_link(
        &mut self,
        remote_router_id: String,
        transit_area_id: String,
    ) -> Result<u32, String> {
        // Validate transit area
        if transit_area_id == "0.0.0.0" {
            return Err("Transit area cannot be the backbone".to_string());
        }
        
        // Check if virtual link already exists
        if self.virtual_links.contains_key(&remote_router_id) {
            return Err(format!("Virtual link to {} already exists", remote_router_id));
        }
        
        // Create new virtual link
        let interface_id = self.next_interface_id;
        self.next_interface_id += 1;
        
        let vlink = VirtualLink::new(
            self.local_router_id.clone(),
            remote_router_id.clone(),
            transit_area_id.clone(),
            interface_id,
        );
        
        self.virtual_links.insert(remote_router_id.clone(), vlink);
        
        console_log!(
            "Configured virtual link {} -> {} through area {}",
            self.local_router_id, remote_router_id, transit_area_id
        );
        
        Ok(interface_id)
    }
    
    /// Remove a virtual link
    pub fn remove_virtual_link(&mut self, remote_router_id: &str) -> bool {
        if self.virtual_links.remove(remote_router_id).is_some() {
            console_log!(
                "Removed virtual link {} -> {}",
                self.local_router_id, remote_router_id
            );
            true
        } else {
            false
        }
    }
    
    /// Process hello received on virtual link
    pub fn process_hello(
        &mut self,
        remote_router_id: &str,
        hello_options: OSPFOptions,
    ) -> bool {
        if let Some(vlink) = self.virtual_links.get_mut(remote_router_id) {
            vlink.last_hello_time = Some(self.current_time);
            vlink.options = hello_options;
            
            // Update neighbor state if needed
            if vlink.neighbor_state == OSPFNeighborState::Down {
                vlink.neighbor_state = OSPFNeighborState::Init;
                console_log!(
                    "Virtual link {} -> {} neighbor state: Down -> Init",
                    self.local_router_id, remote_router_id
                );
            }
            
            true
        } else {
            false
        }
    }
    
    /// Update virtual link based on SPF results
    pub fn update_virtual_link_from_spf(
        &mut self,
        remote_router_id: &str,
        cost: u32,
        next_hop: Option<String>,
    ) -> bool {
        if let Some(vlink) = self.virtual_links.get_mut(remote_router_id) {
            vlink.update_cost(cost);
            
            // Update state based on reachability
            if next_hop.is_some() && cost < u32::MAX {
                if vlink.state == VirtualLinkState::Down {
                    vlink.update_state(VirtualLinkState::PointToPoint);
                }
            } else {
                vlink.update_state(VirtualLinkState::Down);
                vlink.neighbor_state = OSPFNeighborState::Down;
            }
            
            true
        } else {
            false
        }
    }
    
    /// Get all virtual links
    pub fn get_virtual_links(&self) -> &HashMap<String, VirtualLink> {
        &self.virtual_links
    }
    
    /// Get a specific virtual link
    pub fn get_virtual_link(&self, remote_router_id: &str) -> Option<&VirtualLink> {
        self.virtual_links.get(remote_router_id)
    }
    
    /// Get mutable reference to a virtual link
    pub fn get_virtual_link_mut(&mut self, remote_router_id: &str) -> Option<&mut VirtualLink> {
        self.virtual_links.get_mut(remote_router_id)
    }
    
    /// Get virtual link interfaces
    pub fn get_virtual_interfaces(&self) -> Vec<RouterInterface> {
        self.virtual_links
            .values()
            .filter(|vlink| vlink.state == VirtualLinkState::PointToPoint)
            .map(|vlink| vlink.to_interface())
            .collect()
    }
    
    /// Check if any virtual links exist
    pub fn has_virtual_links(&self) -> bool {
        !self.virtual_links.is_empty()
    }
    
    /// Check if a specific virtual link is operational
    pub fn is_virtual_link_up(&self, remote_router_id: &str) -> bool {
        self.virtual_links
            .get(remote_router_id)
            .map(|vlink| vlink.state == VirtualLinkState::PointToPoint)
            .unwrap_or(false)
    }
    
    /// Validate virtual link configuration against area type
    pub fn validate_transit_area(&self, area_id: &str, is_stub: bool) -> Result<(), String> {
        // Check if any virtual links use this area as transit
        for vlink in self.virtual_links.values() {
            if vlink.transit_area_id == area_id && is_stub {
                return Err(format!(
                    "Cannot make area {} stub: it is a transit area for virtual link {} -> {}",
                    area_id, vlink.local_router_id, vlink.remote_router_id
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_virtual_link_creation() {
        let vlink = VirtualLink::new(
            "1.1.1.1".to_string(),
            "2.2.2.2".to_string(),
            "1.0.0.0".to_string(),
            100,
        );
        
        assert_eq!(vlink.state, VirtualLinkState::Down);
        assert_eq!(vlink.neighbor_state, OSPFNeighborState::Down);
        assert_eq!(vlink.transit_area_id, "1.0.0.0");
    }
    
    #[test]
    fn test_virtual_link_activation() {
        let vlink = VirtualLink::new(
            "1.1.1.1".to_string(),
            "2.2.2.2".to_string(),
            "1.0.0.0".to_string(),
            100,
        );
        
        // Both must be ABRs with path
        assert!(vlink.can_activate(true, true, true));
        
        // Not ABR
        assert!(!vlink.can_activate(false, true, true));
        assert!(!vlink.can_activate(true, false, true));
        
        // No path
        assert!(!vlink.can_activate(true, true, false));
        
        // Backbone transit area not allowed
        let vlink_backbone = VirtualLink::new(
            "1.1.1.1".to_string(),
            "2.2.2.2".to_string(),
            "0.0.0.0".to_string(),
            101,
        );
        assert!(!vlink_backbone.can_activate(true, true, true));
    }
    
    #[test]
    fn test_virtual_link_manager() {
        let mut manager = VirtualLinkManager::new("1.1.1.1".to_string(), 100);
        
        // Configure virtual link
        let result = manager.configure_virtual_link(
            "2.2.2.2".to_string(),
            "1.0.0.0".to_string(),
        );
        assert!(result.is_ok());
        let interface_id = result.unwrap();
        assert_eq!(interface_id, 100);
        
        // Cannot use backbone as transit
        let result = manager.configure_virtual_link(
            "3.3.3.3".to_string(),
            "0.0.0.0".to_string(),
        );
        assert!(result.is_err());
        
        // Cannot duplicate
        let result = manager.configure_virtual_link(
            "2.2.2.2".to_string(),
            "1.0.0.0".to_string(),
        );
        assert!(result.is_err());
        
        // Process hello
        manager.update_time(10.0);
        assert!(manager.process_hello("2.2.2.2", OSPFOptions::standard_area_options()));
        
        // Update from SPF
        manager.update_virtual_link_from_spf("2.2.2.2", 10, Some("10.0.0.2".to_string()));
        let vlink = manager.get_virtual_link("2.2.2.2").unwrap();
        assert_eq!(vlink.state, VirtualLinkState::PointToPoint);
        assert_eq!(vlink.cost, 10);
        
        // Test timeout
        manager.update_time(60.0); // Past dead interval
        let vlink = manager.get_virtual_link("2.2.2.2").unwrap();
        assert_eq!(vlink.state, VirtualLinkState::Down);
    }
    
    #[test]
    fn test_transit_area_validation() {
        let mut manager = VirtualLinkManager::new("1.1.1.1".to_string(), 100);
        
        // Configure virtual link through area 1
        manager.configure_virtual_link("2.2.2.2".to_string(), "1.0.0.0".to_string()).unwrap();
        
        // Cannot make area 1 stub
        let result = manager.validate_transit_area("1.0.0.0", true);
        assert!(result.is_err());
        
        // Can make area 2 stub (not transit)
        let result = manager.validate_transit_area("2.0.0.0", true);
        assert!(result.is_ok());
    }
}