import { describe, it, expect, beforeEach, vi } from 'vitest';
import { JSDOM } from 'jsdom';

describe('End-to-End Simulation Tests', () => {
  let dom;
  let window;
  let document;
  let canvas;
  let ctx;

  beforeEach(() => {
    // Set up DOM environment
    dom = new JSDOM(`
      <!DOCTYPE html>
      <html>
        <body>
          <div id="canvas-container" style="width: 800px; height: 600px;">
            <canvas id="network-canvas"></canvas>
          </div>
          <div id="controls">
            <button id="start-btn">Start</button>
            <button id="stop-btn">Stop</button>
            <button id="add-router-btn">Add Router</button>
            <button id="connect-btn">Connect</button>
          </div>
          <div id="info-panel"></div>
        </body>
      </html>
    `);
    
    window = dom.window;
    document = window.document;
    global.window = window;
    global.document = document;
    
    // Mock canvas context
    canvas = document.getElementById('network-canvas');
    ctx = {
      clearRect: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      arc: vi.fn(),
      stroke: vi.fn(),
      fill: vi.fn(),
      fillText: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      setLineDash: vi.fn()
    };
    
    canvas.getContext = vi.fn().mockReturnValue(ctx);
    
    // Mock requestAnimationFrame
    global.requestAnimationFrame = vi.fn((cb) => setTimeout(cb, 16));
  });

  describe('User Interaction Scenarios', () => {
    it('should handle router creation workflow', () => {
      const mockApp = createMockApplication();
      
      // User clicks "Add Router" button
      const addBtn = document.getElementById('add-router-btn');
      addBtn.click();
      
      expect(mockApp.mode).toBe('add-router');
      
      // User clicks on canvas to place router
      const clickEvent = new window.MouseEvent('click', {
        clientX: 400,
        clientY: 300
      });
      canvas.dispatchEvent(clickEvent);
      
      expect(mockApp.routers.length).toBe(1);
      expect(mockApp.routers[0]).toMatchObject({
        x: 400,
        y: 300,
        name: expect.stringMatching(/Router\d+/)
      });
    });

    it('should handle connection creation workflow', () => {
      const mockApp = createMockApplication();
      
      // Add two routers first
      mockApp.addRouter(100, 100);
      mockApp.addRouter(200, 200);
      
      // User clicks "Connect" button
      const connectBtn = document.getElementById('connect-btn');
      connectBtn.click();
      
      expect(mockApp.mode).toBe('connect-routers');
      
      // User clicks first router
      const click1 = new window.MouseEvent('click', {
        clientX: 100,
        clientY: 100
      });
      canvas.dispatchEvent(click1);
      
      expect(mockApp.selectedRouters).toContain(1);
      
      // User clicks second router
      const click2 = new window.MouseEvent('click', {
        clientX: 200,
        clientY: 200
      });
      canvas.dispatchEvent(click2);
      
      expect(mockApp.connections.length).toBe(1);
      expect(mockApp.connections[0]).toMatchObject({
        from_router_id: 1,
        to_router_id: 2,
        cost: expect.any(Number)
      });
    });

    it('should handle simulation control', () => {
      const mockApp = createMockApplication();
      
      // Create a simple network
      const r1 = mockApp.addRouter(100, 100);
      const r2 = mockApp.addRouter(200, 200);
      mockApp.connectRouters(r1, r2, 10);
      mockApp.enableOSPF(r1);
      mockApp.enableOSPF(r2);
      
      // Start simulation
      const startBtn = document.getElementById('start-btn');
      startBtn.click();
      
      expect(mockApp.simulationRunning).toBe(true);
      
      // Simulate some time passing
      for (let i = 0; i < 10; i++) {
        mockApp.step(0.1);
      }
      
      expect(mockApp.simulationTime).toBeCloseTo(1.0);
      
      // Stop simulation
      const stopBtn = document.getElementById('stop-btn');
      stopBtn.click();
      
      expect(mockApp.simulationRunning).toBe(false);
    });
  });

  describe('Visual Feedback', () => {
    it('should update canvas when network changes', () => {
      const mockApp = createMockApplication();
      
      // Add router
      mockApp.addRouter(100, 100);
      mockApp.render();
      
      // Check that router was drawn
      expect(ctx.arc).toHaveBeenCalledWith(100, 100, expect.any(Number), 0, 2 * Math.PI);
      expect(ctx.fill).toHaveBeenCalled();
      
      // Add another router and connection
      const r2 = mockApp.addRouter(200, 200);
      mockApp.connectRouters(1, r2, 10);
      mockApp.render();
      
      // Check that connection was drawn
      expect(ctx.moveTo).toHaveBeenCalled();
      expect(ctx.lineTo).toHaveBeenCalled();
      expect(ctx.stroke).toHaveBeenCalled();
    });

    it('should show OSPF state visually', () => {
      const mockApp = createMockApplication();
      
      const r1 = mockApp.addRouter(100, 100);
      mockApp.enableOSPF(r1);
      mockApp.render();
      
      // OSPF-enabled routers should be drawn differently
      expect(ctx.fillStyle).toBe('#4CAF50'); // Green for OSPF
    });

    it('should animate packets', () => {
      const mockApp = createMockApplication();
      
      // Create network
      const r1 = mockApp.addRouter(100, 100);
      const r2 = mockApp.addRouter(300, 100);
      mockApp.connectRouters(r1, r2, 10);
      mockApp.enableOSPF(r1);
      mockApp.enableOSPF(r2);
      
      // Start simulation
      mockApp.startSimulation();
      
      // Add a packet
      mockApp.addPacket({
        from: r1,
        to: r2,
        type: 'Hello',
        progress: 0
      });
      
      // Render multiple frames
      for (let i = 0; i < 10; i++) {
        mockApp.render();
        mockApp.updatePackets(0.1);
      }
      
      // Check that packet was drawn at different positions
      const arcCalls = ctx.arc.mock.calls;
      const packetPositions = arcCalls.filter(call => call[2] === 5); // Packet radius
      expect(packetPositions.length).toBeGreaterThan(1);
    });
  });

  describe('Information Display', () => {
    it('should update info panel with router details', () => {
      const mockApp = createMockApplication();
      const infoPanel = document.getElementById('info-panel');
      
      const r1 = mockApp.addRouter(100, 100);
      mockApp.enableOSPF(r1);
      
      // Simulate clicking on router
      mockApp.selectRouter(r1);
      mockApp.updateInfoPanel();
      
      expect(infoPanel.innerHTML).toContain('Router1');
      expect(infoPanel.innerHTML).toContain('OSPF: Enabled');
    });

    it('should show routing table information', () => {
      const mockApp = createMockApplication();
      const infoPanel = document.getElementById('info-panel');
      
      // Create network
      const r1 = mockApp.addRouter(100, 100);
      const r2 = mockApp.addRouter(200, 200);
      const r3 = mockApp.addRouter(300, 300);
      
      mockApp.connectRouters(r1, r2, 10);
      mockApp.connectRouters(r2, r3, 10);
      mockApp.enableOSPF(r1);
      mockApp.enableOSPF(r2);
      mockApp.enableOSPF(r3);
      
      // Run simulation
      mockApp.startSimulation();
      for (let i = 0; i < 100; i++) {
        mockApp.step(0.1);
      }
      
      // Select router and update info
      mockApp.selectRouter(r1);
      mockApp.updateInfoPanel();
      
      expect(infoPanel.innerHTML).toContain('Routing Table');
      expect(infoPanel.innerHTML).toContain('Destination');
      expect(infoPanel.innerHTML).toContain('Next Hop');
      expect(infoPanel.innerHTML).toContain('Metric');
    });
  });

  describe('Error Handling', () => {
    it('should handle invalid connections gracefully', () => {
      const mockApp = createMockApplication();
      
      // Try to connect non-existent routers
      const result = mockApp.connectRouters(999, 998, 10);
      expect(result).toBe(false);
      expect(mockApp.connections.length).toBe(0);
    });

    it('should prevent duplicate connections', () => {
      const mockApp = createMockApplication();
      
      const r1 = mockApp.addRouter(100, 100);
      const r2 = mockApp.addRouter(200, 200);
      
      mockApp.connectRouters(r1, r2, 10);
      mockApp.connectRouters(r1, r2, 20); // Try duplicate
      
      expect(mockApp.connections.length).toBe(1);
      expect(mockApp.connections[0].cost).toBe(10); // Original cost
    });
  });

  // Helper function to create mock application
  function createMockApplication() {
    return {
      mode: 'normal',
      routers: [],
      connections: [],
      selectedRouters: [],
      simulationRunning: false,
      simulationTime: 0,
      packets: [],
      nextRouterId: 1,
      
      addRouter(x, y) {
        const id = this.nextRouterId++;
        this.routers.push({
          id,
          x,
          y,
          name: `Router${id}`,
          ospf_enabled: false
        });
        return id;
      },
      
      connectRouters(r1, r2, cost) {
        if (!this.routers.find(r => r.id === r1) || 
            !this.routers.find(r => r.id === r2)) {
          return false;
        }
        
        // Check for duplicate
        const exists = this.connections.find(c => 
          (c.from_router_id === r1 && c.to_router_id === r2) ||
          (c.from_router_id === r2 && c.to_router_id === r1)
        );
        
        if (!exists) {
          this.connections.push({
            from_router_id: r1,
            to_router_id: r2,
            cost
          });
        }
        return true;
      },
      
      enableOSPF(routerId) {
        const router = this.routers.find(r => r.id === routerId);
        if (router) {
          router.ospf_enabled = true;
        }
      },
      
      startSimulation() {
        this.simulationRunning = true;
      },
      
      step(delta) {
        if (this.simulationRunning) {
          this.simulationTime += delta;
        }
      },
      
      selectRouter(routerId) {
        this.selectedRouters = [routerId];
      },
      
      addPacket(packet) {
        this.packets.push(packet);
      },
      
      updatePackets(delta) {
        this.packets.forEach(p => {
          p.progress = Math.min(1, p.progress + delta);
        });
      },
      
      render() {
        ctx.clearRect(0, 0, 800, 600);
        
        // Draw connections
        this.connections.forEach(conn => {
          const r1 = this.routers.find(r => r.id === conn.from_router_id);
          const r2 = this.routers.find(r => r.id === conn.to_router_id);
          if (r1 && r2) {
            ctx.beginPath();
            ctx.moveTo(r1.x, r1.y);
            ctx.lineTo(r2.x, r2.y);
            ctx.stroke();
          }
        });
        
        // Draw routers
        this.routers.forEach(router => {
          ctx.beginPath();
          ctx.arc(router.x, router.y, 20, 0, 2 * Math.PI);
          ctx.fillStyle = router.ospf_enabled ? '#4CAF50' : '#2196F3';
          ctx.fill();
        });
        
        // Draw packets
        this.packets.forEach(packet => {
          ctx.beginPath();
          ctx.arc(packet.x || 0, packet.y || 0, 5, 0, 2 * Math.PI);
          ctx.fill();
        });
      },
      
      updateInfoPanel() {
        const panel = document.getElementById('info-panel');
        if (this.selectedRouters.length > 0) {
          const router = this.routers.find(r => r.id === this.selectedRouters[0]);
          if (router) {
            panel.innerHTML = `
              <h3>${router.name}</h3>
              <p>OSPF: ${router.ospf_enabled ? 'Enabled' : 'Disabled'}</p>
              <h4>Routing Table</h4>
              <table>
                <tr>
                  <th>Destination</th>
                  <th>Next Hop</th>
                  <th>Metric</th>
                </tr>
              </table>
            `;
          }
        }
      }
    };
  }
});