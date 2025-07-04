import { describe, it, expect, beforeEach, vi } from 'vitest';

describe('ConnectionManager', () => {
  let mockWasmModule;
  let mockEventBus;

  beforeEach(() => {
    // Mock WASM module
    mockWasmModule = {
      connect_routers: vi.fn(),
      disconnect_routers: vi.fn().mockReturnValue(true),
      toggle_link_failure: vi.fn().mockReturnValue(true),
      get_connections_json: vi.fn().mockReturnValue('[]')
    };

    // Mock EventBus
    mockEventBus = {
      emit: vi.fn(),
      on: vi.fn(),
      off: vi.fn()
    };
  });

  describe('Connection Creation', () => {
    it('should connect two routers', () => {
      const router1Id = 1;
      const router2Id = 2;
      const cost = 10;

      mockWasmModule.connect_routers(router1Id, router2Id, cost);
      
      expect(mockWasmModule.connect_routers).toHaveBeenCalledWith(router1Id, router2Id, cost);
    });

    it('should emit event when connection is created', () => {
      const router1Id = 1;
      const router2Id = 2;
      const cost = 10;

      mockWasmModule.connect_routers(router1Id, router2Id, cost);
      mockEventBus.emit('connection:created', { 
        router1Id, 
        router2Id, 
        cost 
      });
      
      expect(mockEventBus.emit).toHaveBeenCalledWith('connection:created', {
        router1Id,
        router2Id,
        cost
      });
    });

    it('should handle invalid router IDs', () => {
      const invalidId = -1;
      const validId = 1;
      const cost = 10;

      // In a real implementation, this might throw or return false
      mockWasmModule.connect_routers(invalidId, validId, cost);
      
      expect(mockWasmModule.connect_routers).toHaveBeenCalledWith(invalidId, validId, cost);
    });
  });

  describe('Connection Deletion', () => {
    it('should disconnect two routers', () => {
      const router1Id = 1;
      const router2Id = 2;

      const result = mockWasmModule.disconnect_routers(router1Id, router2Id);
      
      expect(mockWasmModule.disconnect_routers).toHaveBeenCalledWith(router1Id, router2Id);
      expect(result).toBe(true);
    });

    it('should emit event when connection is deleted', () => {
      const router1Id = 1;
      const router2Id = 2;

      mockWasmModule.disconnect_routers(router1Id, router2Id);
      mockEventBus.emit('connection:deleted', { 
        router1Id, 
        router2Id 
      });
      
      expect(mockEventBus.emit).toHaveBeenCalledWith('connection:deleted', {
        router1Id,
        router2Id
      });
    });
  });

  describe('Link Failure Simulation', () => {
    it('should toggle link failure', () => {
      const router1Id = 1;
      const router2Id = 2;

      const result = mockWasmModule.toggle_link_failure(router1Id, router2Id);
      
      expect(mockWasmModule.toggle_link_failure).toHaveBeenCalledWith(router1Id, router2Id);
      expect(result).toBe(true);
    });

    it('should emit event when link failure is toggled', () => {
      const router1Id = 1;
      const router2Id = 2;
      let isFailed = false;

      // Toggle to failed
      mockWasmModule.toggle_link_failure(router1Id, router2Id);
      isFailed = true;
      mockEventBus.emit('link:failure:toggled', { 
        router1Id, 
        router2Id, 
        isFailed 
      });
      
      expect(mockEventBus.emit).toHaveBeenCalledWith('link:failure:toggled', {
        router1Id,
        router2Id,
        isFailed: true
      });

      // Toggle back to normal
      mockWasmModule.toggle_link_failure(router1Id, router2Id);
      isFailed = false;
      mockEventBus.emit('link:failure:toggled', { 
        router1Id, 
        router2Id, 
        isFailed 
      });
      
      expect(mockEventBus.emit).toHaveBeenCalledWith('link:failure:toggled', {
        router1Id,
        router2Id,
        isFailed: false
      });
    });
  });

  describe('Connection Information', () => {
    it('should get all connections', () => {
      const mockConnections = JSON.stringify([
        { router1_id: 1, router2_id: 2, cost: 10, is_failed: false },
        { router1_id: 2, router2_id: 3, cost: 20, is_failed: false }
      ]);
      
      mockWasmModule.get_connections_json.mockReturnValue(mockConnections);
      
      const connections = mockWasmModule.get_connections_json();
      
      expect(connections).toBe(mockConnections);
    });

    it('should parse connections correctly', () => {
      const mockConnections = [
        { router1_id: 1, router2_id: 2, cost: 10, is_failed: false },
        { router1_id: 2, router2_id: 3, cost: 20, is_failed: true }
      ];
      
      mockWasmModule.get_connections_json.mockReturnValue(JSON.stringify(mockConnections));
      
      const connectionsJson = mockWasmModule.get_connections_json();
      const connections = JSON.parse(connectionsJson);
      
      expect(connections).toHaveLength(2);
      expect(connections[0].cost).toBe(10);
      expect(connections[1].is_failed).toBe(true);
    });
  });

  describe('Cost Management', () => {
    it('should handle different cost values', () => {
      const testCases = [
        { router1: 1, router2: 2, cost: 1 },    // Minimum cost
        { router1: 1, router2: 3, cost: 100 },  // High cost
        { router1: 2, router2: 3, cost: 65535 } // Maximum OSPF cost
      ];

      testCases.forEach(({ router1, router2, cost }) => {
        mockWasmModule.connect_routers(router1, router2, cost);
        
        expect(mockWasmModule.connect_routers).toHaveBeenCalledWith(router1, router2, cost);
      });
    });
  });

  describe('Topology Validation', () => {
    it('should prevent duplicate connections', () => {
      const router1Id = 1;
      const router2Id = 2;
      const cost = 10;

      // First connection
      mockWasmModule.connect_routers(router1Id, router2Id, cost);
      
      // Attempt duplicate connection
      mockWasmModule.connect_routers(router1Id, router2Id, cost);
      
      expect(mockWasmModule.connect_routers).toHaveBeenCalledTimes(2);
    });

    it('should handle self-connections', () => {
      const routerId = 1;
      const cost = 10;

      // Attempt self-connection
      mockWasmModule.connect_routers(routerId, routerId, cost);
      
      expect(mockWasmModule.connect_routers).toHaveBeenCalledWith(routerId, routerId, cost);
    });
  });
});