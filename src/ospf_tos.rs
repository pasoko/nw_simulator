use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// OSPFv2 Type of Service (TOS) Support (RFC 2328 Section 7)
/// 
/// This module implements TOS-based routing support for OSPFv2.
/// Although TOS routing is deprecated in modern OSPF implementations,
/// it's included here for RFC 2328 compliance.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TOSValue(u8);

impl TOSValue {
    /// Create a new TOS value (0-127)
    pub fn new(value: u8) -> Result<Self, String> {
        if value > 127 {
            Err(format!("TOS value {} exceeds maximum of 127", value))
        } else {
            Ok(TOSValue(value))
        }
    }
    
    /// Get the raw TOS value
    pub fn value(&self) -> u8 {
        self.0
    }
    
    /// Normal service (default)
    pub fn normal() -> Self {
        TOSValue(0)
    }
    
    /// Minimize monetary cost
    pub fn minimize_cost() -> Self {
        TOSValue(1)
    }
    
    /// Maximize reliability
    pub fn maximize_reliability() -> Self {
        TOSValue(2)
    }
    
    /// Maximize throughput
    pub fn maximize_throughput() -> Self {
        TOSValue(4)
    }
    
    /// Minimize delay
    pub fn minimize_delay() -> Self {
        TOSValue(8)
    }
    
    /// Check if this is normal service
    pub fn is_normal(&self) -> bool {
        self.0 == 0
    }
}

impl Default for TOSValue {
    fn default() -> Self {
        Self::normal()
    }
}

/// TOS metric for a specific TOS value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TOSMetric {
    pub tos: TOSValue,
    pub metric: u32,
}

impl TOSMetric {
    pub fn new(tos: TOSValue, metric: u32) -> Self {
        TOSMetric { tos, metric }
    }
    
    /// Create a normal TOS metric
    pub fn normal(metric: u32) -> Self {
        TOSMetric {
            tos: TOSValue::normal(),
            metric,
        }
    }
}

/// TOS capabilities for a router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TOSCapabilities {
    /// T-bit in Options field - indicates TOS support
    pub tos_support_enabled: bool,
    
    /// Supported TOS values
    pub supported_tos_values: Vec<TOSValue>,
    
    /// Per-interface TOS metrics
    pub interface_tos_metrics: HashMap<u32, Vec<TOSMetric>>,
}

impl TOSCapabilities {
    pub fn new() -> Self {
        TOSCapabilities {
            tos_support_enabled: false,
            supported_tos_values: vec![TOSValue::normal()],
            interface_tos_metrics: HashMap::new(),
        }
    }
    
    /// Enable TOS support
    pub fn enable_tos_support(&mut self) {
        self.tos_support_enabled = true;
    }
    
    /// Disable TOS support
    pub fn disable_tos_support(&mut self) {
        self.tos_support_enabled = false;
        self.supported_tos_values = vec![TOSValue::normal()];
        self.interface_tos_metrics.clear();
    }
    
    /// Add a supported TOS value
    pub fn add_supported_tos(&mut self, tos: TOSValue) {
        if !self.supported_tos_values.contains(&tos) {
            self.supported_tos_values.push(tos);
        }
    }
    
    /// Remove a supported TOS value (cannot remove normal TOS)
    pub fn remove_supported_tos(&mut self, tos: TOSValue) {
        if !tos.is_normal() {
            self.supported_tos_values.retain(|&t| t != tos);
        }
    }
    
    /// Check if a TOS value is supported
    pub fn is_tos_supported(&self, tos: &TOSValue) -> bool {
        self.supported_tos_values.contains(tos)
    }
    
    /// Set TOS metrics for an interface
    pub fn set_interface_tos_metrics(&mut self, interface_id: u32, metrics: Vec<TOSMetric>) {
        // Ensure all metrics are for supported TOS values
        let filtered_metrics: Vec<TOSMetric> = metrics
            .into_iter()
            .filter(|m| self.is_tos_supported(&m.tos))
            .collect();
        
        if !filtered_metrics.is_empty() {
            self.interface_tos_metrics.insert(interface_id, filtered_metrics);
        }
    }
    
    /// Get TOS metric for a specific interface and TOS
    pub fn get_interface_tos_metric(&self, interface_id: u32, tos: &TOSValue) -> Option<u32> {
        self.interface_tos_metrics
            .get(&interface_id)
            .and_then(|metrics| {
                metrics.iter()
                    .find(|m| m.tos == *tos)
                    .map(|m| m.metric)
            })
    }
    
    /// Get all TOS metrics for an interface
    pub fn get_interface_all_tos_metrics(&self, interface_id: u32) -> Vec<TOSMetric> {
        self.interface_tos_metrics
            .get(&interface_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }
}

impl Default for TOSCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// TOS-specific routing table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TOSRoutingEntry {
    pub destination: String,
    pub mask: String,
    pub tos: TOSValue,
    pub cost: u32,
    pub next_hop: String,
    pub outgoing_interface: u32,
    pub advertising_router: String,
}

/// TOS routing table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TOSRoutingTable {
    /// Routes indexed by destination and TOS
    routes: HashMap<(String, TOSValue), TOSRoutingEntry>,
}

impl TOSRoutingTable {
    pub fn new() -> Self {
        TOSRoutingTable {
            routes: HashMap::new(),
        }
    }
    
    /// Add or update a TOS route
    pub fn add_route(&mut self, entry: TOSRoutingEntry) {
        let key = (entry.destination.clone(), entry.tos);
        self.routes.insert(key, entry);
    }
    
    /// Remove a TOS route
    pub fn remove_route(&mut self, destination: &str, tos: TOSValue) {
        self.routes.remove(&(destination.to_string(), tos));
    }
    
    /// Get a specific TOS route
    pub fn get_route(&self, destination: &str, tos: TOSValue) -> Option<&TOSRoutingEntry> {
        self.routes.get(&(destination.to_string(), tos))
    }
    
    /// Get all routes for a destination
    pub fn get_all_tos_routes(&self, destination: &str) -> Vec<&TOSRoutingEntry> {
        self.routes
            .iter()
            .filter(|((dest, _), _)| dest == destination)
            .map(|(_, entry)| entry)
            .collect()
    }
    
    /// Get all routes
    pub fn get_all_routes(&self) -> Vec<&TOSRoutingEntry> {
        self.routes.values().collect()
    }
    
    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
    }
    
    /// Get route count
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl Default for TOSRoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// TOS field in Router LSA link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLinkTOS {
    pub tos: TOSValue,
    pub metric: u16,
}

/// Extended Router LSA link with TOS support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLinkWithTOS {
    pub link_id: String,
    pub link_data: String,
    pub link_type: u8,
    pub num_tos: u8,
    pub metric: u16,
    pub tos_metrics: Vec<RouterLinkTOS>,
}

impl RouterLinkWithTOS {
    pub fn new(
        link_id: String,
        link_data: String,
        link_type: u8,
        metric: u16,
    ) -> Self {
        RouterLinkWithTOS {
            link_id,
            link_data,
            link_type,
            num_tos: 0,
            metric,
            tos_metrics: Vec::new(),
        }
    }
    
    /// Add a TOS metric
    pub fn add_tos_metric(&mut self, tos: TOSValue, metric: u16) {
        if !tos.is_normal() {
            self.tos_metrics.push(RouterLinkTOS { tos, metric });
            self.num_tos = self.tos_metrics.len() as u8;
        }
    }
    
    /// Get metric for a specific TOS
    pub fn get_tos_metric(&self, tos: &TOSValue) -> u16 {
        if tos.is_normal() {
            self.metric
        } else {
            self.tos_metrics
                .iter()
                .find(|t| t.tos == *tos)
                .map(|t| t.metric)
                .unwrap_or(self.metric)
        }
    }
}

/// Summary LSA with TOS support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryLSAWithTOS {
    pub network_mask: String,
    pub metric: u32,
    pub tos_metrics: Vec<TOSMetric>,
}

impl SummaryLSAWithTOS {
    pub fn new(network_mask: String, metric: u32) -> Self {
        SummaryLSAWithTOS {
            network_mask,
            metric,
            tos_metrics: Vec::new(),
        }
    }
    
    /// Add a TOS metric
    pub fn add_tos_metric(&mut self, tos: TOSValue, metric: u32) {
        if !tos.is_normal() {
            self.tos_metrics.push(TOSMetric { tos, metric });
        }
    }
    
    /// Get metric for a specific TOS
    pub fn get_tos_metric(&self, tos: &TOSValue) -> u32 {
        if tos.is_normal() {
            self.metric
        } else {
            self.tos_metrics
                .iter()
                .find(|t| t.tos == *tos)
                .map(|t| t.metric)
                .unwrap_or(self.metric)
        }
    }
}

/// AS-External LSA with TOS support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASExternalLSAWithTOS {
    pub network_mask: String,
    pub metric_type: u8, // E1 or E2
    pub metric: u32,
    pub forwarding_address: String,
    pub external_route_tag: u32,
    pub tos_metrics: Vec<ExternalTOSMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTOSMetric {
    pub tos: TOSValue,
    pub metric_type: u8,
    pub metric: u32,
    pub forwarding_address: String,
    pub external_route_tag: u32,
}

impl ASExternalLSAWithTOS {
    pub fn new(
        network_mask: String,
        metric_type: u8,
        metric: u32,
        forwarding_address: String,
        external_route_tag: u32,
    ) -> Self {
        ASExternalLSAWithTOS {
            network_mask,
            metric_type,
            metric,
            forwarding_address,
            external_route_tag,
            tos_metrics: Vec::new(),
        }
    }
    
    /// Add a TOS metric
    pub fn add_tos_metric(
        &mut self,
        tos: TOSValue,
        metric_type: u8,
        metric: u32,
        forwarding_address: String,
        external_route_tag: u32,
    ) {
        if !tos.is_normal() {
            self.tos_metrics.push(ExternalTOSMetric {
                tos,
                metric_type,
                metric,
                forwarding_address,
                external_route_tag,
            });
        }
    }
}