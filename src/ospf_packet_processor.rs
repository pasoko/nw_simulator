use std::collections::HashMap;
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket, DatabaseDescriptionPacket, 
    LinkStateRequestPacket, LinkStateUpdatePacket, LSA, LSAHeader, LSARequest};
use crate::router::{OSPFNeighborState, LSAType, LSAHeader as RouterLSAHeader};
use crate::protocol::{ProtocolPacket, PacketEvent};
use crate::console_log;

/// Database Description Exchange State
#[derive(Clone)]
pub struct DDExchangeState {
    pub dd_seq_num: u32,
    pub is_master: bool,
    pub last_received_dd_seq: u32,
    pub lsa_headers_to_request: Vec<RouterLSAHeader>,
    pub lsa_headers_sent: Vec<RouterLSAHeader>,
    pub lsa_headers_to_send: Vec<RouterLSAHeader>,
    pub dd_exchange_done: bool,
}

/// OSPF Packet Processing
/// 
/// Handles all OSPF packet processing including:
/// - Hello packet processing
/// - Database Description exchange
/// - LSA Request/Update/Acknowledgment processing
/// - Packet generation
pub struct OSPFPacketProcessor {
    router_id: String,
    area_id: String,
    hello_interval: u16,
    dead_interval: u32,
    dd_sequence_number: u32,
    neighbor_dd_state: HashMap<u32, DDExchangeState>,
}

impl OSPFPacketProcessor {
    pub fn new(router_id: String, area_id: String) -> Self {
        OSPFPacketProcessor {
            router_id,
            area_id,
            hello_interval: 10,
            dead_interval: 40,
            dd_sequence_number: 0x80000001,
            neighbor_dd_state: HashMap::new(),
        }
    }
    
    pub fn generate_hello_packet(&self, active_neighbors: &[String]) -> HelloPacket {
        console_log!("Router {} generating Hello packet with {} neighbors: {:?}", 
            self.router_id, active_neighbors.len(), active_neighbors);
        
        HelloPacket {
            network_mask: "255.255.255.252".to_string(),
            hello_interval: self.hello_interval,
            options: 0x02, // E-bit set
            router_priority: 1,
            router_dead_interval: self.dead_interval,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: active_neighbors.to_vec(),
        }
    }
    
    pub fn process_hello_packet(&mut self, packet: &HelloPacket, from_router_id: u32) 
        -> (bool, Vec<String>) {
        console_log!("Router {} received Hello from router {}", 
            self.router_id, from_router_id);
        
        (true, packet.neighbors.clone())
    }
    
    pub fn process_dd_packet(&mut self, packet: &DatabaseDescriptionPacket, from_router_id: u32, 
        current_state: OSPFNeighborState) -> (Option<OSPFNeighborState>, bool, Vec<RouterLSAHeader>) {
        
        match current_state {
            OSPFNeighborState::ExStart => {
                // Master/Slave negotiation
                let our_router_id_num = self.router_id.split('.').last()
                    .unwrap_or("0").parse::<u32>().unwrap_or(0);
                let is_master = our_router_id_num > from_router_id;
                
                // Initialize DD exchange state
                let dd_state = DDExchangeState {
                    dd_seq_num: if is_master { self.dd_sequence_number } else { packet.dd_sequence_number },
                    is_master,
                    last_received_dd_seq: packet.dd_sequence_number,
                    lsa_headers_to_request: Vec::new(),
                    lsa_headers_sent: Vec::new(),
                    lsa_headers_to_send: Vec::new(),
                    dd_exchange_done: false,
                };
                self.neighbor_dd_state.insert(from_router_id, dd_state);
                
                (Some(OSPFNeighborState::Exchange), true, Vec::new())
            }
            OSPFNeighborState::Exchange => {
                let mut should_send_dd = false;
                let mut new_state = None;
                let mut lsa_headers_to_request = Vec::new();
                
                if let Some(dd_state) = self.neighbor_dd_state.get_mut(&from_router_id) {
                    // Validate sequence number
                    let more_flag = packet.flags & 0x02 != 0;  // M bit
                    let init_flag = packet.flags & 0x04 != 0;  // I bit
                    let ms_flag = packet.flags & 0x01 != 0;   // MS bit
                    
                    console_log!("Router {} received DD from {}: M={}, I={}, MS={}, seq={}", 
                        self.router_id, from_router_id, more_flag, init_flag, ms_flag, packet.dd_sequence_number);
                    
                    // Update last received sequence number
                    dd_state.last_received_dd_seq = packet.dd_sequence_number;
                    
                    // Process received LSA headers
                    for lsa_header in &packet.lsa_headers {
                        let router_lsa_header = RouterLSAHeader {
                            ls_age: lsa_header.age,
                            ls_type: match lsa_header.lsa_type {
                                1 => LSAType::RouterLSA,
                                2 => LSAType::NetworkLSA,
                                3 => LSAType::SummaryLSA,
                                5 => LSAType::ASExternalLSA,
                                _ => LSAType::RouterLSA,
                            },
                            link_state_id: lsa_header.link_state_id.clone(),
                            advertising_router: lsa_header.advertising_router.clone(),
                            ls_sequence_number: lsa_header.sequence_number,
                            ls_checksum: lsa_header.checksum,
                            length: lsa_header.length,
                        };
                        dd_state.lsa_headers_to_request.push(router_lsa_header);
                    }
                    
                    // Increment sequence number for next DD packet if we are master
                    if dd_state.is_master {
                        dd_state.dd_seq_num = dd_state.dd_seq_num.wrapping_add(1);
                        should_send_dd = true;
                    } else {
                        // Slave responds with received sequence number
                        should_send_dd = true;
                    }
                    
                    // Check for completion of DD exchange
                    // Exchange is done when both sides have sent DD packets with M=0
                    if !more_flag && !init_flag {
                        dd_state.dd_exchange_done = true;
                        
                        // Only transition to next state if we've also sent our last DD
                        if dd_state.lsa_headers_to_send.is_empty() {
                            if dd_state.lsa_headers_to_request.is_empty() {
                                new_state = Some(OSPFNeighborState::Full);
                                console_log!("Router {} neighbor {} moving to Full (no LSAs to request)", 
                                    self.router_id, from_router_id);
                            } else {
                                new_state = Some(OSPFNeighborState::Loading);
                                lsa_headers_to_request = dd_state.lsa_headers_to_request.clone();
                                console_log!("Router {} neighbor {} moving to Loading (requesting {} LSAs)", 
                                    self.router_id, from_router_id, lsa_headers_to_request.len());
                            }
                        }
                    }
                }
                
                (new_state, should_send_dd, lsa_headers_to_request)
            }
            _ => (None, false, Vec::new())
        }
    }
    
    pub fn process_lsr_packet(&self, packet: &LinkStateRequestPacket, lsa_database: &HashMap<String, crate::router::LSA>) 
        -> Vec<LSA> {
        let mut lsas_to_send = Vec::new();
        
        for request in &packet.requests {
            let key = format!("{}:{}:{}", 
                request.lsa_type, 
                request.link_state_id, 
                request.advertising_router
            );
            
            if let Some(lsa) = lsa_database.get(&key) {
                let lsa_for_packet = LSA {
                    header: LSAHeader {
                        age: lsa.header.ls_age,
                        options: 0x02,
                        lsa_type: lsa.header.ls_type.clone() as u8,
                        link_state_id: lsa.header.link_state_id.clone(),
                        advertising_router: lsa.header.advertising_router.clone(),
                        sequence_number: lsa.header.ls_sequence_number,
                        checksum: lsa.header.ls_checksum,
                        length: lsa.header.length,
                    },
                    data: lsa.data.clone()
                };
                lsas_to_send.push(lsa_for_packet);
            }
        }
        
        lsas_to_send
    }
    
    pub fn process_lsu_packet(&mut self, packet: &LinkStateUpdatePacket, from_router_id: u32) 
        -> (Vec<crate::router::LSA>, Vec<LSAHeader>, bool) {
        let mut updated_lsas = Vec::new();
        let mut ack_headers = Vec::new();
        let mut neighbor_to_full = false;
        
        console_log!("Router {} processing LSU from router {} with {} LSAs", 
            self.router_id, from_router_id, packet.lsas.len());
        
        for lsa in &packet.lsas {
            // Convert from OSPF packet LSA to router LSA
            let router_lsa_header = RouterLSAHeader {
                ls_age: lsa.header.age,
                ls_type: match lsa.header.lsa_type {
                    1 => LSAType::RouterLSA,
                    2 => LSAType::NetworkLSA,
                    3 => LSAType::SummaryLSA,
                    5 => LSAType::ASExternalLSA,
                    _ => LSAType::RouterLSA,
                },
                link_state_id: lsa.header.link_state_id.clone(),
                advertising_router: lsa.header.advertising_router.clone(),
                ls_sequence_number: lsa.header.sequence_number,
                ls_checksum: lsa.header.checksum,
                length: lsa.header.length,
            };
            
            let router_lsa = crate::router::LSA {
                header: router_lsa_header,
                data: lsa.data.clone(),
            };
            
            updated_lsas.push(router_lsa);
            ack_headers.push(lsa.header.clone());
        }
        
        // Check if we can move to Full state
        if let Some(dd_state) = self.neighbor_dd_state.get_mut(&from_router_id) {
            // Remove received LSAs from request list
            for lsa in &packet.lsas {
                dd_state.lsa_headers_to_request.retain(|req| {
                    !(req.ls_type.clone() as u8 == lsa.header.lsa_type &&
                      req.link_state_id == lsa.header.link_state_id &&
                      req.advertising_router == lsa.header.advertising_router)
                });
            }
            
            if dd_state.lsa_headers_to_request.is_empty() {
                neighbor_to_full = true;
            }
        }
        
        (updated_lsas, ack_headers, neighbor_to_full)
    }
    
    pub fn create_dd_packet_event(&mut self, to_router_id: u32, lsa_database: &HashMap<String, crate::router::LSA>) -> PacketEvent {
        let mut flags = 0u8;
        let dd_seq_num;
        let mut lsa_headers_to_send = Vec::new();
        
        if let Some(dd_state) = self.neighbor_dd_state.get_mut(&to_router_id) {
            // In Exchange state - send LSA headers
            if dd_state.lsa_headers_to_send.is_empty() && dd_state.lsa_headers_sent.is_empty() {
                // First DD packet - prepare all LSA headers to send
                dd_state.lsa_headers_to_send = lsa_database.values().map(|lsa| {
                    RouterLSAHeader {
                        ls_age: lsa.header.ls_age,
                        ls_type: lsa.header.ls_type.clone(),
                        link_state_id: lsa.header.link_state_id.clone(),
                        advertising_router: lsa.header.advertising_router.clone(),
                        ls_sequence_number: lsa.header.ls_sequence_number,
                        ls_checksum: lsa.header.ls_checksum,
                        length: lsa.header.length,
                    }
                }).collect();
            }
            
            // Send a batch of LSA headers (max 100 per packet for example)
            let batch_size = 100;
            let headers_to_send: Vec<_> = dd_state.lsa_headers_to_send
                .drain(..dd_state.lsa_headers_to_send.len().min(batch_size))
                .collect();
            
            // Convert to packet format
            lsa_headers_to_send = headers_to_send.iter().map(|h| {
                LSAHeader {
                    age: h.ls_age,
                    options: 0x02,
                    lsa_type: h.ls_type.clone() as u8,
                    link_state_id: h.link_state_id.clone(),
                    advertising_router: h.advertising_router.clone(),
                    sequence_number: h.ls_sequence_number,
                    checksum: h.ls_checksum,
                    length: h.length,
                }
            }).collect();
            
            // Track sent headers
            dd_state.lsa_headers_sent.extend(headers_to_send);
            
            if dd_state.is_master {
                flags |= 0x01; // MS bit
                dd_seq_num = dd_state.dd_seq_num;
            } else {
                // Slave uses received sequence number
                dd_seq_num = dd_state.last_received_dd_seq;
            }
            
            // Set More bit only if we have more headers to send
            if !dd_state.lsa_headers_to_send.is_empty() {
                flags |= 0x02; // M bit
            }
        } else {
            // Initial DD packet for ExStart state
            flags = 0x07; // I, M, MS bits for initial negotiation
            dd_seq_num = self.dd_sequence_number;
        }
        
        let dd_packet = DatabaseDescriptionPacket {
            interface_mtu: 1500,
            options: 0x02,
            flags,
            dd_sequence_number: dd_seq_num,
            lsa_headers: lsa_headers_to_send,
        };
        
        let ospf_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::DatabaseDescription,
            router_id: self.router_id.clone(),
            area_id: self.area_id.clone(),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::DatabaseDescription(dd_packet),
        };
        
        let from_id = self.router_id.split('.').last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        
        PacketEvent {
            timestamp: 0.0,
            from_router_id: from_id,
            to_router_id,
            packet: ProtocolPacket::OSPF(ospf_packet),
        }
    }
    
    pub fn create_lsr_packet_event(&self, to_router_id: u32, lsa_headers: &[RouterLSAHeader]) -> PacketEvent {
        let requests: Vec<LSARequest> = lsa_headers.iter().map(|header| {
            LSARequest {
                lsa_type: header.ls_type.clone() as u8,
                link_state_id: header.link_state_id.clone(),
                advertising_router: header.advertising_router.clone(),
            }
        }).collect();
        
        let lsr_packet = LinkStateRequestPacket { requests };
        
        let ospf_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::LinkStateRequest,
            router_id: self.router_id.clone(),
            area_id: self.area_id.clone(),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::LinkStateRequest(lsr_packet),
        };
        
        let from_id = self.router_id.split('.').last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        
        PacketEvent {
            timestamp: 0.0,
            from_router_id: from_id,
            to_router_id,
            packet: ProtocolPacket::OSPF(ospf_packet),
        }
    }
    
    pub fn create_lsu_packet_event(&self, to_router_id: u32, lsas: &[LSA]) -> PacketEvent {
        let lsu_packet = LinkStateUpdatePacket {
            lsas: lsas.to_vec(),
        };
        
        let ospf_packet = OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::LinkStateUpdate,
            router_id: self.router_id.clone(),
            area_id: self.area_id.clone(),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::LinkStateUpdate(lsu_packet),
        };
        
        let from_id = self.router_id.split('.').last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        
        console_log!("Router {} creating LSU packet for router {} with {} LSAs", 
            self.router_id, to_router_id, lsas.len());
        
        PacketEvent {
            timestamp: 0.0,
            from_router_id: from_id,
            to_router_id,
            packet: ProtocolPacket::OSPF(ospf_packet),
        }
    }
}