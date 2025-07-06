import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { JSDOM } from 'jsdom';

// Mock the dependencies before importing canvas-renderer
vi.mock('./state-manager.js', () => ({
  default: {
    routers: [],
    connections: [],
    simulationRunning: false,
    packetVisualizer: null,
    canvasRenderer: null,
    simulator: {
      get_router_summary_json: vi.fn().mockReturnValue('{}')
    },
    findRouterById: vi.fn(),
    isRouterSelected: vi.fn().mockReturnValue(false),
    getMode: vi.fn().mockReturnValue('normal'),
    selectedRouters: [],
    draggingRouter: null,
    lastMouseX: 0,
    lastMouseY: 0
  }
}));

vi.mock('./theme-manager.js', () => ({
  default: {
    isDarkMode: vi.fn().mockReturnValue(false)
  }
}));

vi.mock('./animation-effects.js', () => ({
  default: {
    drawAnimations: vi.fn()
  }
}));

vi.mock('./router-icon.js', () => ({
  RouterIcon: vi.fn().mockImplementation(() => ({
    draw: vi.fn(),
    isPointInRouter: vi.fn().mockReturnValue(false),
    iconSize: 50
  }))
}));

// Now import canvas-renderer after mocks are set up
import canvasRenderer from './canvas-renderer.js';
import stateManager from './state-manager.js';
import themeManager from './theme-manager.js';

describe.skip('CanvasRenderer', () => {
  let mockCanvas;
  let mockCtx;
  let mockStateManager;
  let mockPacketVisualizer;
  let mockContainer;

  beforeEach(() => {
    // Clear all mocks
    vi.clearAllMocks();
    
    // Set up DOM environment
    const dom = new JSDOM('<!DOCTYPE html><div id="canvas-container"></div>');
    global.document = dom.window.document;
    global.window = dom.window;

    // Mock container
    mockContainer = document.getElementById('canvas-container');
    Object.defineProperty(mockContainer, 'clientWidth', { value: 800, configurable: true });
    Object.defineProperty(mockContainer, 'clientHeight', { value: 600, configurable: true });

    // Mock canvas context
    mockCtx = {
      clearRect: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      arc: vi.fn(),
      stroke: vi.fn(),
      fill: vi.fn(),
      fillText: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      setLineDash: vi.fn(),
      measureText: vi.fn().mockReturnValue({ width: 50 }),
      quadraticCurveTo: vi.fn(),
      closePath: vi.fn(),
      createLinearGradient: vi.fn().mockReturnValue({
        addColorStop: vi.fn()
      }),
      strokeStyle: '',
      fillStyle: '',
      lineWidth: 1,
      font: '',
      textAlign: 'left',
      textBaseline: 'alphabetic',
      shadowColor: '',
      shadowBlur: 0,
      shadowOffsetX: 0,
      shadowOffsetY: 0,
      lineCap: 'butt'
    };

    // Mock canvas
    mockCanvas = {
      width: 800,
      height: 600,
      getContext: vi.fn().mockReturnValue(mockCtx)
    };

    // Mock packet visualizer
    mockPacketVisualizer = {
      draw: vi.fn()
    };

    // Update stateManager mock values
    stateManager.routers = [];
    stateManager.connections = [];
    stateManager.simulationRunning = false;
    stateManager.packetVisualizer = mockPacketVisualizer;
    stateManager.canvasRenderer = null;
    stateManager.selectedRouters = [];
    stateManager.draggingRouter = null;
    stateManager.lastMouseX = 0;
    stateManager.lastMouseY = 0;
  });

  describe('Initialization', () => {
    it('should initialize canvas renderer correctly', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      expect(canvasRenderer.canvas).toBe(mockCanvas);
      expect(canvasRenderer.ctx).toBe(mockCtx);
      expect(canvasRenderer.packetVisualizer).toBe(mockPacketVisualizer);
      expect(stateManager.canvasRenderer).toBe(canvasRenderer);
    });

    it('should setup canvas dimensions on init', () => {
      
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      expect(mockCanvas.width).toBe(800);
      expect(mockCanvas.height).toBe(600);
    });

    it('should handle window resize', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      canvasRenderer.render = vi.fn();
      
      // Change container size
      Object.defineProperty(mockContainer, 'clientWidth', { value: 1024 });
      Object.defineProperty(mockContainer, 'clientHeight', { value: 768 });
      
      // Trigger resize event
      window.dispatchEvent(new Event('resize'));
      
      expect(mockCanvas.width).toBe(1024);
      expect(mockCanvas.height).toBe(768);
      expect(canvasRenderer.render).toHaveBeenCalled();
    });
  });

  describe('Rendering', () => {
    it('should clear canvas before rendering', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      // Test that render method doesn't throw
      expect(() => canvasRenderer.render()).not.toThrow();
      
      // Canvas operations will be called through the render process
      expect(mockCtx.clearRect).toHaveBeenCalled();
    });

    it('should update router summaries when not simulating', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      stateManager.routers = [
        { id: 1, name: 'Router1', x: 100, y: 100 },
        { id: 2, name: 'Router2', x: 200, y: 200 }
      ];
      stateManager.simulationRunning = false;
      stateManager.simulator.get_router_summary_json
        .mockReturnValueOnce('{"id":1,"status":"ok"}')
        .mockReturnValueOnce('{"id":2,"status":"ok"}');
      
      canvasRenderer.render();
      
      // Check that summaries were retrieved
      expect(stateManager.simulator.get_router_summary_json).toHaveBeenCalled();
    });

    it('should not update router summaries when simulating', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      stateManager.simulationRunning = true;
      stateManager.routers = [{ id: 1, name: 'Router1' }];
      
      canvasRenderer.render();
      
      expect(stateManager.simulator.get_router_summary_json).not.toHaveBeenCalled();
    });

    it('should render packet visualizer if available', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      canvasRenderer.packetVisualizer = mockPacketVisualizer;
      
      canvasRenderer.render();
      
      expect(mockPacketVisualizer.draw).toHaveBeenCalled();
    });
  });

  describe('Connection Drawing', () => {
    it('should draw connections between routers', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router1 = { id: 1, x: 100, y: 100 };
      const router2 = { id: 2, x: 300, y: 100 };
      const connection = {
        from_router_id: 1,
        to_router_id: 2,
        from_interface_id: 1,
        to_interface_id: 2,
        cost: 10,
        is_failed: false
      };
      
      // Call drawConnection directly
      canvasRenderer.drawConnection(router1, router2, connection);
      
      // Should draw connection line
      expect(mockCtx.beginPath).toHaveBeenCalled();
      expect(mockCtx.moveTo).toHaveBeenCalled();
      expect(mockCtx.lineTo).toHaveBeenCalled();
      expect(mockCtx.stroke).toHaveBeenCalled();
    });

    it('should style failed connections differently', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router1 = { id: 1, x: 100, y: 100 };
      const router2 = { id: 2, x: 300, y: 100 };
      
      stateManager.connections = [{
        from_router_id: 1,
        to_router_id: 2,
        cost: 10,
        is_failed: true
      }];
      stateManager.findRouterById
        .mockReturnValueOnce(router1)
        .mockReturnValueOnce(router2);
      
      canvasRenderer.drawConnection(router1, router2, stateManager.connections[0]);
      
      // Failed connections are styled with dashed lines
      expect(mockCtx.setLineDash).toHaveBeenCalledWith([8, 4]);
      // Should draw main line and arrows
      expect(mockCtx.stroke.mock.calls.length).toBeGreaterThanOrEqual(2);
    });

    it('should draw bidirectional arrows', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router1 = { id: 1, x: 100, y: 100 };
      const router2 = { id: 2, x: 300, y: 100 };
      const connection = { cost: 10, is_failed: false };
      
      canvasRenderer.drawConnection(router1, router2, connection);
      
      // Should draw arrows at both ends
      const strokeCalls = mockCtx.stroke.mock.calls.length;
      expect(strokeCalls).toBeGreaterThanOrEqual(2); // Main line + arrows
    });

    it('should draw interface labels', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router1 = { id: 1, x: 100, y: 100 };
      const router2 = { id: 2, x: 300, y: 100 };
      const connection = {
        from_interface_id: 1,
        to_interface_id: 2,
        cost: 10,
        is_failed: false
      };
      
      canvasRenderer.drawConnection(router1, router2, connection);
      
      // Should draw interface labels
      expect(mockCtx.fillText).toHaveBeenCalledWith(expect.stringContaining('if1'), expect.any(Number), expect.any(Number));
      expect(mockCtx.fillText).toHaveBeenCalledWith(expect.stringContaining('if2'), expect.any(Number), expect.any(Number));
    });

    it('should draw cost label at midpoint', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router1 = { id: 1, x: 100, y: 100 };
      const router2 = { id: 2, x: 300, y: 100 };
      const connection = { cost: 10, is_failed: false };
      
      canvasRenderer.drawConnection(router1, router2, connection);
      
      // Should draw cost label at midpoint
      expect(mockCtx.fillText).toHaveBeenCalledWith('Cost: 10', 200, 100);
    });
  });

  describe('Router Drawing', () => {
    it('should draw routers with correct styling', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router = {
        id: 1,
        name: 'Router1',
        x: 100,
        y: 100,
        ospf_enabled: false,
        is_failed: false
      };
      
      canvasRenderer.drawRouter(router);
      
      // RouterIcon instance should have been called
      expect(canvasRenderer.routerIcon.draw).toHaveBeenCalled();
    });

    it('should style OSPF-enabled routers differently', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router = {
        id: 1,
        name: 'Router1',
        x: 100,
        y: 100,
        ospf_enabled: true,
        is_failed: false
      };
      
      canvasRenderer.drawRouter(router);
      
      // Check that routerIcon was called with correct state
      expect(canvasRenderer.routerIcon.draw).toHaveBeenCalledWith(
        mockCtx,
        100,
        100,
        1,
        expect.objectContaining({
          ospfEnabled: true
        })
      );
    });

    it('should style failed routers with red', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router = {
        id: 1,
        name: 'Router1',
        x: 100,
        y: 100,
        ospf_enabled: true,
        is_failed: true
      };
      
      canvasRenderer.drawRouter(router);
      
      // Check that routerIcon was called with failed state
      expect(canvasRenderer.routerIcon.draw).toHaveBeenCalledWith(
        mockCtx,
        100,
        100,
        1,
        expect.objectContaining({
          failed: true
        })
      );
    });

    it('should highlight selected routers in connect mode', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      stateManager.getMode.mockReturnValue('connect-routers');
      stateManager.isRouterSelected.mockReturnValue(true);
      stateManager.selectedRouters = [1];
      
      const router = { id: 1, name: 'Router1', x: 100, y: 100 };
      
      canvasRenderer.drawRouter(router);
      
      // Should draw selection ring
      // The selection ring is drawn at radius 35
      expect(mockCtx.arc).toHaveBeenCalledWith(100, 100, 35, 0, 2 * Math.PI);
    });

    it('should mark router as dragging in state', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      const router = { id: 1, name: 'Router1', x: 100, y: 100 };
      stateManager.draggingRouter = router;
      
      canvasRenderer.drawRouter(router);
      
      // Check that routerIcon was called with dragging state
      expect(canvasRenderer.routerIcon.draw).toHaveBeenCalledWith(
        mockCtx,
        100,
        100,
        1,
        expect.objectContaining({
          dragging: true
        })
      );
    });
  });

  describe('Packet Statistics', () => {
    it('should draw packet statistics when simulation is running', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      // Set simulation as running
      stateManager.simulationRunning = true;
      
      // Mock packet visualizer methods
      const mockVis = {
        draw: vi.fn(),
        getPacketsByType: vi.fn().mockReturnValue({
          'Hello': 2,
          'DD': 1,
          'LSRequest': 1
        }),
        getActivePacketCount: vi.fn().mockReturnValue(4),
        packetConfigs: {
          'Hello': { color: '#ff0000' },
          'DD': { color: '#00ff00' },
          'LSRequest': { color: '#0000ff' }
        }
      };
      canvasRenderer.packetVisualizer = mockVis;
      
      // Call drawPacketStats directly
      canvasRenderer.drawPacketStats();
      
      // Check that packet stats were drawn
      expect(mockCtx.fillText).toHaveBeenCalledWith(expect.stringContaining('Active Packets: 4'), expect.any(Number), expect.any(Number));
      expect(mockCtx.fillText).toHaveBeenCalledWith(expect.stringContaining('Hello: 2'), expect.any(Number), expect.any(Number));
    });
  });

  describe('Edge Cases', () => {
    it('should handle missing context gracefully', () => {
      
      canvasRenderer.canvas = mockCanvas;
      canvasRenderer.ctx = null;
      
      // Should not throw
      expect(() => canvasRenderer.render()).not.toThrow();
    });

    it('should handle empty routers and connections', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      stateManager.routers = [];
      stateManager.connections = [];
      
      // Should not throw
      expect(() => canvasRenderer.render()).not.toThrow();
    });

    it('should handle missing router in connection', () => {
      
      canvasRenderer.init(mockCanvas, mockCtx);
      
      stateManager.connections = [{
        from_router_id: 1,
        to_router_id: 2,
        cost: 10
      }];
      stateManager.findRouterById.mockReturnValue(null);
      
      // Should not throw
      expect(() => canvasRenderer.render()).not.toThrow();
    });
  });

});
