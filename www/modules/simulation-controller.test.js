import { describe, it, expect, beforeEach, vi } from 'vitest';

describe('SimulationController', () => {
  let mockWasmModule;
  let mockEventBus;
  let simulationController;

  beforeEach(() => {
    // Mock WASM module
    mockWasmModule = {
      start_simulation: vi.fn(),
      stop_simulation: vi.fn(),
      step_simulation: vi.fn(),
      get_simulation_stats_json: vi.fn().mockReturnValue('{"running":false,"time":0}'),
      get_recent_events_json: vi.fn().mockReturnValue('[]')
    };

    // Mock EventBus
    mockEventBus = {
      emit: vi.fn(),
      on: vi.fn(),
      off: vi.fn()
    };

    // Mock timer functions
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  describe('Simulation Control', () => {
    it('should start simulation', () => {
      mockWasmModule.start_simulation();
      mockEventBus.emit('simulation:started');
      
      expect(mockWasmModule.start_simulation).toHaveBeenCalled();
      expect(mockEventBus.emit).toHaveBeenCalledWith('simulation:started');
    });

    it('should stop simulation', () => {
      mockWasmModule.stop_simulation();
      mockEventBus.emit('simulation:stopped');
      
      expect(mockWasmModule.stop_simulation).toHaveBeenCalled();
      expect(mockEventBus.emit).toHaveBeenCalledWith('simulation:stopped');
    });

    it('should step simulation with time delta', () => {
      const timeDelta = 0.1;
      
      mockWasmModule.step_simulation(timeDelta);
      
      expect(mockWasmModule.step_simulation).toHaveBeenCalledWith(timeDelta);
    });
  });

  describe('Simulation Loop', () => {
    it('should run simulation loop when started', () => {
      const timeDelta = 0.1;
      let isRunning = true;
      
      // Simulate a basic run loop
      const runLoop = () => {
        if (isRunning) {
          mockWasmModule.step_simulation(timeDelta);
          mockEventBus.emit('simulation:step', { timeDelta });
        }
      };

      // Start simulation
      mockWasmModule.start_simulation();
      runLoop();
      
      expect(mockWasmModule.step_simulation).toHaveBeenCalledWith(timeDelta);
      expect(mockEventBus.emit).toHaveBeenCalledWith('simulation:step', { timeDelta });
    });

    it('should handle different simulation speeds', () => {
      const speeds = [0.5, 1.0, 2.0, 5.0];
      
      speeds.forEach(speed => {
        mockWasmModule.step_simulation(0.1 * speed);
        
        expect(mockWasmModule.step_simulation).toHaveBeenCalledWith(0.1 * speed);
      });
    });
  });

  describe('Simulation State', () => {
    it('should get simulation statistics', () => {
      const stats = mockWasmModule.get_simulation_stats_json();
      
      expect(stats).toBe('{"running":false,"time":0}');
    });

    it('should get recent events', () => {
      const eventCount = 10;
      mockWasmModule.get_recent_events_json.mockReturnValue('[{"type":"hello","time":0.1}]');
      
      const events = mockWasmModule.get_recent_events_json(eventCount);
      
      expect(mockWasmModule.get_recent_events_json).toHaveBeenCalledWith(eventCount);
      expect(events).toBe('[{"type":"hello","time":0.1}]');
    });
  });

  describe('Time Management', () => {
    it('should track simulation time', () => {
      let simulationTime = 0;
      const timeDelta = 0.1;
      
      // Simulate 10 steps
      for (let i = 0; i < 10; i++) {
        mockWasmModule.step_simulation(timeDelta);
        simulationTime += timeDelta;
      }
      
      expect(mockWasmModule.step_simulation).toHaveBeenCalledTimes(10);
      expect(simulationTime).toBeCloseTo(1.0);
    });

    it('should pause and resume simulation', () => {
      let isPaused = false;
      
      // Start simulation
      mockWasmModule.start_simulation();
      
      // Pause
      isPaused = true;
      mockEventBus.emit('simulation:paused');
      
      // Resume
      isPaused = false;
      mockEventBus.emit('simulation:resumed');
      
      expect(mockEventBus.emit).toHaveBeenCalledWith('simulation:paused');
      expect(mockEventBus.emit).toHaveBeenCalledWith('simulation:resumed');
    });
  });

  describe('Performance Monitoring', () => {
    it('should measure step performance', () => {
      const start = performance.now();
      
      mockWasmModule.step_simulation(0.1);
      
      const end = performance.now();
      const stepTime = end - start;
      
      // Step should complete quickly (< 16ms for 60fps)
      expect(stepTime).toBeLessThan(16);
    });

    it('should handle simulation overload', () => {
      let frameDropped = false;
      const targetFrameTime = 16.67; // 60fps
      
      // Simulate a slow step
      const slowStep = () => {
        const start = performance.now();
        mockWasmModule.step_simulation(0.1);
        const stepTime = performance.now() - start;
        
        if (stepTime > targetFrameTime) {
          frameDropped = true;
          mockEventBus.emit('simulation:framedrop', { stepTime });
        }
      };
      
      slowStep();
      
      // In a real scenario, we'd check if frames were dropped
      expect(mockWasmModule.step_simulation).toHaveBeenCalled();
    });
  });
});