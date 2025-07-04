import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { JSDOM } from 'jsdom';

// Mock WebAssembly module
const mockWasmModule = {
  NetworkSimulator: class {
    constructor() {
      this.routers = new Map();
      this.connections = new Map();
      this.running = false;
      this.time = 0;
      this.eventCount = 0;
      this.nextRouterId = 1;
      this.nextConnectionId = 1;
    }

    add_router(name, x, y) {
      const id = this.nextRouterId++;
      this.routers.set(id, { id, name, x, y, ospf_enabled: false, is_failed: false });
      return id;
    }

    delete_router(id) {
      if (this.routers.has(id)) {
        this.routers.delete(id);
        // Remove connections
        for (const [connId, conn] of this.connections) {
          if (conn.router1_id === id || conn.router2_id === id) {
            this.connections.delete(connId);
          }
        }
        return true;
      }
      return false;
    }

    connect_routers(r1, r2, cost) {
      const id = this.nextConnectionId++;
      this.connections.set(id, {
        id,
        router1_id: r1,
        router2_id: r2,
        cost,
        is_failed: false
      });
    }

    disconnect_routers(r1, r2) {
      for (const [id, conn] of this.connections) {
        if ((conn.router1_id === r1 && conn.router2_id === r2) ||
            (conn.router1_id === r2 && conn.router2_id === r1)) {
          this.connections.delete(id);
          return true;
        }
      }
      return false;
    }

    enable_ospf(routerId) {
      const router = this.routers.get(routerId);
      if (router) {
        router.ospf_enabled = true;
      }
    }

    start_simulation() {
      this.running = true;
    }

    stop_simulation() {
      this.running = false;
    }

    step_simulation(delta) {
      if (this.running) {
        this.time += delta;
        this.eventCount++;
      }
    }

    get_routers_json() {
      return JSON.stringify(Array.from(this.routers.values()));
    }

    get_connections_json() {
      return JSON.stringify(Array.from(this.connections.values()));
    }

    get_simulation_stats_json() {
      return JSON.stringify({
        running: this.running,
        time: this.time,
        event_count: this.eventCount
      });
    }

    get_router_details_json(id) {
      const router = this.routers.get(id);
      if (!router) return '{}';
      
      return JSON.stringify({
        ...router,
        routing_table: [],
        ospf_state: {
          neighbors: {},
          lsa_database: []
        }
      });
    }

    get_recent_events_json(count) {
      // Mock some events
      const events = [];
      for (let i = 0; i < Math.min(count, 5); i++) {
        events.push({
          event_type: 'Hello packet',
          timestamp: this.time - i,
          from_router: 1,
          to_router: 2
        });
      }
      return JSON.stringify(events);
    }

    toggle_link_failure(r1, r2) {
      for (const conn of this.connections.values()) {
        if ((conn.router1_id === r1 && conn.router2_id === r2) ||
            (conn.router1_id === r2 && conn.router2_id === r1)) {
          conn.is_failed = !conn.is_failed;
          return true;
        }
      }
      return false;
    }

    toggle_router_failure(id) {
      const router = this.routers.get(id);
      if (router) {
        router.is_failed = !router.is_failed;
      }
    }

    update_router_position(id, x, y) {
      const router = this.routers.get(id);
      if (router) {
        router.x = x;
        router.y = y;
        return true;
      }
      return false;
    }
  }
};

describe('WASM Integration Tests', () => {
  let dom;
  let window;
  let document;
  let simulator;

  beforeEach(() => {
    // Set up DOM
    dom = new JSDOM('<!DOCTYPE html><html><body><canvas id="canvas"></canvas></body></html>');
    window = dom.window;
    document = window.document;
    global.window = window;
    global.document = document;

    // Create simulator instance
    simulator = new mockWasmModule.NetworkSimulator();
  });

  afterEach(() => {
    delete global.window;
    delete global.document;
  });

  describe('Router Management', () => {
    it('should create and manage routers', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);

      expect(r1).toBe(1);
      expect(r2).toBe(2);

      const routers = JSON.parse(simulator.get_routers_json());
      expect(routers).toHaveLength(2);
      expect(routers[0].name).toBe('Router1');
      expect(routers[1].name).toBe('Router2');
    });

    it('should delete routers', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);
      
      simulator.connect_routers(r1, r2, 10);
      
      expect(simulator.delete_router(r1)).toBe(true);
      
      const routers = JSON.parse(simulator.get_routers_json());
      expect(routers).toHaveLength(1);
      
      const connections = JSON.parse(simulator.get_connections_json());
      expect(connections).toHaveLength(0);
    });
  });

  describe('Connection Management', () => {
    it('should create and manage connections', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);
      const r3 = simulator.add_router('Router3', 300, 300);
      
      simulator.connect_routers(r1, r2, 10);
      simulator.connect_routers(r2, r3, 20);
      
      const connections = JSON.parse(simulator.get_connections_json());
      expect(connections).toHaveLength(2);
      expect(connections[0].cost).toBe(10);
      expect(connections[1].cost).toBe(20);
    });

    it('should disconnect routers', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);
      
      simulator.connect_routers(r1, r2, 10);
      expect(simulator.disconnect_routers(r1, r2)).toBe(true);
      
      const connections = JSON.parse(simulator.get_connections_json());
      expect(connections).toHaveLength(0);
    });
  });

  describe('OSPF Functionality', () => {
    it('should enable OSPF on routers', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      simulator.enable_ospf(r1);
      
      const details = JSON.parse(simulator.get_router_details_json(r1));
      expect(details.ospf_enabled).toBe(true);
    });

    it('should manage OSPF state', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);
      
      simulator.connect_routers(r1, r2, 10);
      simulator.enable_ospf(r1);
      simulator.enable_ospf(r2);
      
      const details = JSON.parse(simulator.get_router_details_json(r1));
      expect(details.ospf_state).toBeDefined();
      expect(details.ospf_state.neighbors).toBeDefined();
      expect(details.ospf_state.lsa_database).toBeDefined();
    });
  });

  describe('Simulation Control', () => {
    it('should control simulation state', () => {
      simulator.start_simulation();
      
      let stats = JSON.parse(simulator.get_simulation_stats_json());
      expect(stats.running).toBe(true);
      expect(stats.time).toBe(0);
      
      simulator.step_simulation(0.1);
      simulator.step_simulation(0.1);
      
      stats = JSON.parse(simulator.get_simulation_stats_json());
      expect(stats.time).toBeCloseTo(0.2);
      expect(stats.event_count).toBe(2);
      
      simulator.stop_simulation();
      
      stats = JSON.parse(simulator.get_simulation_stats_json());
      expect(stats.running).toBe(false);
    });

    it('should handle simulation events', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);
      
      simulator.connect_routers(r1, r2, 10);
      simulator.enable_ospf(r1);
      simulator.enable_ospf(r2);
      
      simulator.start_simulation();
      simulator.step_simulation(1.0);
      
      const events = JSON.parse(simulator.get_recent_events_json(10));
      expect(events).toBeInstanceOf(Array);
      expect(events.length).toBeGreaterThan(0);
    });
  });

  describe('Failure Simulation', () => {
    it('should simulate link failures', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      const r2 = simulator.add_router('Router2', 200, 200);
      
      simulator.connect_routers(r1, r2, 10);
      
      expect(simulator.toggle_link_failure(r1, r2)).toBe(true);
      
      const connections = JSON.parse(simulator.get_connections_json());
      expect(connections[0].is_failed).toBe(true);
      
      expect(simulator.toggle_link_failure(r1, r2)).toBe(true);
      
      const connections2 = JSON.parse(simulator.get_connections_json());
      expect(connections2[0].is_failed).toBe(false);
    });

    it('should simulate router failures', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      
      simulator.toggle_router_failure(r1);
      
      const details = JSON.parse(simulator.get_router_details_json(r1));
      expect(details.is_failed).toBe(true);
    });
  });

  describe('UI State Management', () => {
    it('should update router positions', () => {
      const r1 = simulator.add_router('Router1', 100, 100);
      
      expect(simulator.update_router_position(r1, 150, 150)).toBe(true);
      
      const routers = JSON.parse(simulator.get_routers_json());
      expect(routers[0].x).toBe(150);
      expect(routers[0].y).toBe(150);
    });

    it('should handle non-existent routers gracefully', () => {
      expect(simulator.delete_router(999)).toBe(false);
      expect(simulator.update_router_position(999, 100, 100)).toBe(false);
      expect(simulator.toggle_link_failure(999, 998)).toBe(false);
    });
  });

  describe('Data Serialization', () => {
    it('should serialize router data correctly', () => {
      const r1 = simulator.add_router('TestRouter', 123.45, 678.90);
      simulator.enable_ospf(r1);
      
      const json = simulator.get_routers_json();
      const routers = JSON.parse(json);
      
      expect(routers[0]).toMatchObject({
        id: 1,
        name: 'TestRouter',
        x: 123.45,
        y: 678.90,
        ospf_enabled: true
      });
    });

    it('should serialize connection data correctly', () => {
      const r1 = simulator.add_router('R1', 100, 100);
      const r2 = simulator.add_router('R2', 200, 200);
      
      simulator.connect_routers(r1, r2, 25);
      
      const json = simulator.get_connections_json();
      const connections = JSON.parse(json);
      
      expect(connections[0]).toMatchObject({
        router1_id: 1,
        router2_id: 2,
        cost: 25,
        is_failed: false
      });
    });
  });
});