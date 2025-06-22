use serde::{Serialize, Deserialize};
use crate::ospf::OSPFPacket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolPacket {
    OSPF(OSPFPacket),
}

pub trait RoutingProtocol {
    fn process_packet(&mut self, packet: ProtocolPacket, from_router_id: u32);
    fn generate_packets(&mut self) -> Vec<(u32, ProtocolPacket)>;
    fn get_protocol_name(&self) -> &str;
    fn start(&mut self);
    fn stop(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketEvent {
    pub timestamp: f64,
    pub from_router_id: u32,
    pub to_router_id: u32,
    pub packet: ProtocolPacket,
}

#[derive(Debug)]
pub struct ProtocolEngine {
    pub events: Vec<PacketEvent>,
    pub current_time: f64,
}

impl ProtocolEngine {
    pub fn new() -> Self {
        ProtocolEngine {
            events: Vec::new(),
            current_time: 0.0,
        }
    }

    pub fn schedule_event(&mut self, event: PacketEvent) {
        let insert_pos = self.events
            .binary_search_by(|e| e.timestamp.partial_cmp(&event.timestamp).unwrap())
            .unwrap_or_else(|pos| pos);
        self.events.insert(insert_pos, event);
    }

    pub fn process_next_event(&mut self) -> Option<PacketEvent> {
        if self.events.is_empty() {
            return None;
        }
        let event = self.events.remove(0);
        self.current_time = event.timestamp;
        Some(event)
    }

    pub fn advance_time(&mut self, delta: f64) {
        self.current_time += delta;
    }
}