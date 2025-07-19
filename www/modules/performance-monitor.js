/**
 * Performance Monitor Module
 * 
 * Provides real-time performance monitoring and tuning capabilities
 * for the OSPF network simulator.
 */

import eventLogger from './event-logger.js';

class PerformanceMonitor {
    constructor() {
        this.simulator = null;
        this.updateInterval = null;
        this.metricsHistory = [];
        this.maxHistorySize = 100;
        this.lastUpdateTime = 0;
        this.isVisible = false;
    }
    
    /**
     * Initialize the performance monitor
     * @param {Object} simulator - The network simulator instance
     */
    init(simulator) {
        this.simulator = simulator;
        this.createUI();
        this.attachEventHandlers();
        
        // Auto-tune on initialization
        this.autoTune();
        
        eventLogger.log('Performance monitor initialized');
    }
    
    /**
     * Create the performance monitor UI
     */
    createUI() {
        // Create performance panel
        const panel = document.createElement('div');
        panel.id = 'performance-panel';
        panel.className = 'performance-panel hidden';
        panel.innerHTML = `
            <div class="performance-header">
                <h3>Performance Monitor</h3>
                <button class="close-btn" title="Close">×</button>
            </div>
            <div class="performance-content">
                <div class="profile-section">
                    <h4>Performance Profile</h4>
                    <select id="performance-profile">
                        <option value="default">Default</option>
                        <option value="small_network">Small Network (< 50 routers)</option>
                        <option value="medium_network">Medium Network (50-200 routers)</option>
                        <option value="large_network">Large Network (> 200 routers)</option>
                        <option value="real_time">Real-time Priority</option>
                    </select>
                    <button id="apply-profile-btn">Apply Profile</button>
                    <button id="auto-tune-btn">Auto-Tune</button>
                </div>
                
                <div class="metrics-section">
                    <h4>Performance Metrics</h4>
                    <div id="performance-metrics">
                        <div class="metric-item">
                            <span class="metric-label">Total Packets:</span>
                            <span class="metric-value" id="total-packets">0</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">SPF Calculations:</span>
                            <span class="metric-value" id="spf-calculations">0</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">Dropped Packets:</span>
                            <span class="metric-value" id="dropped-packets">0</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">Max LSA DB Size:</span>
                            <span class="metric-value" id="lsa-db-size">0</span>
                        </div>
                        <div class="metric-item">
                            <span class="metric-label">Router Count:</span>
                            <span class="metric-value" id="router-count">0</span>
                        </div>
                    </div>
                    <button id="reset-metrics-btn">Reset Metrics</button>
                </div>
                
                <div class="recommendations-section">
                    <h4>Performance Recommendations</h4>
                    <div id="performance-recommendations" class="recommendations-list">
                        <p class="no-recommendations">No recommendations at this time.</p>
                    </div>
                </div>
                
                <div class="chart-section">
                    <h4>Performance Trends</h4>
                    <canvas id="performance-chart" width="400" height="200"></canvas>
                </div>
            </div>
        `;
        
        document.body.appendChild(panel);
        
        // Create toggle button in sidebar
        const toggleBtn = document.createElement('button');
        toggleBtn.id = 'performance-toggle';
        toggleBtn.className = 'sidebar-button';
        toggleBtn.innerHTML = '📊 Performance';
        toggleBtn.title = 'Toggle Performance Monitor';
        
        const sidebar = document.getElementById('sidebar');
        if (sidebar) {
            // Add before the event log section
            const eventLogSection = sidebar.querySelector('.event-log-section');
            if (eventLogSection) {
                sidebar.insertBefore(toggleBtn, eventLogSection);
            } else {
                sidebar.appendChild(toggleBtn);
            }
        }
    }
    
    /**
     * Attach event handlers
     */
    attachEventHandlers() {
        // Toggle button
        const toggleBtn = document.getElementById('performance-toggle');
        if (toggleBtn) {
            toggleBtn.addEventListener('click', () => this.toggle());
        }
        
        // Close button
        const closeBtn = document.querySelector('#performance-panel .close-btn');
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this.hide());
        }
        
        // Apply profile button
        const applyBtn = document.getElementById('apply-profile-btn');
        if (applyBtn) {
            applyBtn.addEventListener('click', () => this.applyProfile());
        }
        
        // Auto-tune button
        const autoTuneBtn = document.getElementById('auto-tune-btn');
        if (autoTuneBtn) {
            autoTuneBtn.addEventListener('click', () => this.autoTune());
        }
        
        // Reset metrics button
        const resetBtn = document.getElementById('reset-metrics-btn');
        if (resetBtn) {
            resetBtn.addEventListener('click', () => this.resetMetrics());
        }
    }
    
    /**
     * Toggle performance monitor visibility
     */
    toggle() {
        if (this.isVisible) {
            this.hide();
        } else {
            this.show();
        }
    }
    
    /**
     * Show performance monitor
     */
    show() {
        const panel = document.getElementById('performance-panel');
        if (panel) {
            panel.classList.remove('hidden');
            this.isVisible = true;
            
            // Start metrics update
            this.startMetricsUpdate();
            
            // Initialize chart
            this.initChart();
        }
    }
    
    /**
     * Hide performance monitor
     */
    hide() {
        const panel = document.getElementById('performance-panel');
        if (panel) {
            panel.classList.add('hidden');
            this.isVisible = false;
            
            // Stop metrics update
            this.stopMetricsUpdate();
        }
    }
    
    /**
     * Apply selected performance profile
     */
    async applyProfile() {
        const select = document.getElementById('performance-profile');
        if (!select || !this.simulator) return;
        
        const profile = select.value;
        
        try {
            await this.simulator.set_performance_profile(profile);
            eventLogger.log(`Applied performance profile: ${profile}`);
            
            // Update metrics after profile change
            this.updateMetrics();
        } catch (error) {
            eventLogger.log(`Failed to apply performance profile: ${error}`);
        }
    }
    
    /**
     * Auto-tune performance based on network size
     */
    autoTune() {
        if (!this.simulator) return;
        
        try {
            this.simulator.auto_tune_performance();
            eventLogger.log('Auto-tuned performance settings');
            
            // Update metrics after auto-tuning
            this.updateMetrics();
        } catch (error) {
            eventLogger.log(`Failed to auto-tune performance: ${error}`);
        }
    }
    
    /**
     * Reset performance metrics
     */
    resetMetrics() {
        if (!this.simulator) return;
        
        try {
            this.simulator.reset_performance_metrics();
            this.metricsHistory = [];
            eventLogger.log('Reset performance metrics');
            
            // Update display
            this.updateMetrics();
        } catch (error) {
            eventLogger.log(`Failed to reset metrics: ${error}`);
        }
    }
    
    /**
     * Start automatic metrics update
     */
    startMetricsUpdate() {
        // Update every 2 seconds
        this.updateInterval = setInterval(() => {
            this.updateMetrics();
        }, 2000);
        
        // Initial update
        this.updateMetrics();
    }
    
    /**
     * Stop automatic metrics update
     */
    stopMetricsUpdate() {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
            this.updateInterval = null;
        }
    }
    
    /**
     * Update performance metrics display
     */
    async updateMetrics() {
        if (!this.simulator || !this.isVisible) return;
        
        try {
            // Get metrics
            const metricsJson = this.simulator.get_performance_metrics();
            const metrics = JSON.parse(metricsJson);
            
            // Update aggregate metrics display
            if (metrics.aggregate) {
                this.updateMetricValue('total-packets', metrics.aggregate.total_packets_processed);
                this.updateMetricValue('spf-calculations', metrics.aggregate.total_spf_calculations);
                this.updateMetricValue('dropped-packets', metrics.aggregate.total_dropped_packets);
                this.updateMetricValue('lsa-db-size', metrics.aggregate.max_lsa_database_size);
                this.updateMetricValue('router-count', metrics.aggregate.router_count);
                
                // Add to history for chart
                this.addToHistory(metrics.aggregate);
            }
            
            // Get and display recommendations
            const recommendationsJson = this.simulator.get_performance_recommendations();
            const recommendations = JSON.parse(recommendationsJson);
            this.updateRecommendations(recommendations);
            
            // Update chart
            this.updateChart();
            
        } catch (error) {
            console.error('Failed to update metrics:', error);
        }
    }
    
    /**
     * Update a metric value display
     */
    updateMetricValue(elementId, value) {
        const element = document.getElementById(elementId);
        if (element) {
            element.textContent = this.formatNumber(value);
        }
    }
    
    /**
     * Format number for display
     */
    formatNumber(value) {
        if (value >= 1000000) {
            return (value / 1000000).toFixed(1) + 'M';
        } else if (value >= 1000) {
            return (value / 1000).toFixed(1) + 'K';
        }
        return value.toString();
    }
    
    /**
     * Update recommendations display
     */
    updateRecommendations(recommendations) {
        const container = document.getElementById('performance-recommendations');
        if (!container) return;
        
        if (recommendations.length === 0) {
            container.innerHTML = '<p class="no-recommendations">No recommendations at this time.</p>';
        } else {
            container.innerHTML = recommendations.map(rec => 
                `<div class="recommendation-item">⚠️ ${rec}</div>`
            ).join('');
        }
    }
    
    /**
     * Add metrics to history
     */
    addToHistory(metrics) {
        const timestamp = Date.now();
        
        this.metricsHistory.push({
            timestamp,
            packets: metrics.total_packets_processed,
            spf: metrics.total_spf_calculations,
            dropped: metrics.total_dropped_packets
        });
        
        // Limit history size
        if (this.metricsHistory.length > this.maxHistorySize) {
            this.metricsHistory.shift();
        }
    }
    
    /**
     * Initialize performance chart
     */
    initChart() {
        const canvas = document.getElementById('performance-chart');
        if (!canvas) return;
        
        const ctx = canvas.getContext('2d');
        this.chartContext = ctx;
        
        // Set canvas size
        canvas.width = canvas.offsetWidth;
        canvas.height = canvas.offsetHeight;
    }
    
    /**
     * Update performance chart
     */
    updateChart() {
        if (!this.chartContext || this.metricsHistory.length < 2) return;
        
        const ctx = this.chartContext;
        const canvas = ctx.canvas;
        const width = canvas.width;
        const height = canvas.height;
        
        // Clear canvas
        ctx.clearRect(0, 0, width, height);
        
        // Draw grid
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 0.5;
        ctx.beginPath();
        for (let i = 0; i <= 4; i++) {
            const y = (height / 4) * i;
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
        }
        ctx.stroke();
        
        // Calculate scales
        const maxPackets = Math.max(...this.metricsHistory.map(h => h.packets), 1);
        const xScale = width / (this.maxHistorySize - 1);
        const yScale = height / maxPackets;
        
        // Draw packet rate line
        ctx.strokeStyle = '#4CAF50';
        ctx.lineWidth = 2;
        ctx.beginPath();
        this.metricsHistory.forEach((point, index) => {
            const x = index * xScale;
            const y = height - (point.packets * yScale);
            if (index === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        });
        ctx.stroke();
        
        // Draw labels
        ctx.fillStyle = '#ccc';
        ctx.font = '12px monospace';
        ctx.textAlign = 'left';
        ctx.fillText('Packets Processed', 10, 20);
        ctx.textAlign = 'right';
        ctx.fillText(this.formatNumber(maxPackets), width - 10, 20);
    }
}

// Export singleton instance
const performanceMonitor = new PerformanceMonitor();
export default performanceMonitor;