/**
 * Main Application Entry Point (Refactored)
 * 
 * This is the simplified main entry point that coordinates the various modules.
 * The complex functionality has been extracted into focused modules.
 * 
 * 初期化フロー:
 * 1. テーママネージャーの初期化（ダーク/ライトモード設定）
 * 2. アプリケーションモジュールの初期化（WebAssembly読み込み含む）
 * 3. リファクタリング版エンジンの条件付き初期化（URLパラメータ依存）
 * 4. グローバルエラーハンドリングの設定
 */

import appInitializer from './modules/app-initializer.js';
import eventLogger from './modules/event-logger.js';
import RefactoredOSPFAdapter from './modules/refactored-ospf-adapter.js';
import themeManager from './modules/theme-manager.js';

async function main() {
    try {
        // 1. テーママネージャーの初期化（システム設定に基づくテーマ適用）
        themeManager.init();
        
        // 2. メインアプリケーションの初期化
        eventLogger.log('Starting OSPF Network Simulator...');
        window.appInitializer = appInitializer; // Make globally accessible for debugging
        await appInitializer.init();
        eventLogger.log('Application modules initialized successfully');
        
        // 3. リファクタリング版エンジンの条件付き初期化
        // URLに ?refactored=true が含まれる場合のみ有効化
        if (window.location.search.includes('refactored=true')) {
            await initializeRefactoredEngine();
        }
        
        eventLogger.log('Application loaded successfully');
        
    } catch (error) {
        console.error('Failed to start application:', error);
        eventLogger.log(`Failed to start application: ${error.message}`);
    }
}

// Error handling for unhandled errors
window.addEventListener('error', (event) => {
    console.error('Unhandled error:', event.error);
    eventLogger.log(`Unhandled error: ${event.error.message}`);
});

window.addEventListener('unhandledrejection', (event) => {
    console.error('Unhandled promise rejection:', event.reason);
    eventLogger.log(`Unhandled promise rejection: ${event.reason}`);
    event.preventDefault();
});

// Initialize refactored OSPF engine for testing
async function initializeRefactoredEngine() {
    try {
        eventLogger.log('Initializing refactored OSPF engine...');
        
        // Get the simulator instance from app initializer
        const simulator = window.networkSimulator;
        if (!simulator) {
            throw new Error('Simulator not available');
        }
        
        // Create adapter
        const adapter = new RefactoredOSPFAdapter(simulator);
        window.refactoredOSPF = adapter;
        
        // Initialize with default config
        const success = await adapter.initialize();
        if (!success) {
            throw new Error('Failed to initialize refactored engine');
        }
        
        // Enable all features for testing
        adapter.enableFeature('all');
        
        // Register event handlers
        adapter.on('NeighborStateChanged', (event) => {
            eventLogger.log(`[Refactored] Neighbor state changed: ${JSON.stringify(event.details)}`);
        });
        
        adapter.on('LSAReceived', (event) => {
            eventLogger.log(`[Refactored] LSA received: ${JSON.stringify(event.details)}`);
        });
        
        adapter.on('SPFRequired', (event) => {
            eventLogger.log(`[Refactored] SPF required: ${event.details.reason}`);
        });
        
        eventLogger.log('Refactored OSPF engine initialized successfully');
        
        // Add UI indicator
        const indicator = document.createElement('div');
        indicator.style.cssText = 'position: fixed; top: 10px; right: 10px; background: #4CAF50; color: white; padding: 5px 10px; border-radius: 3px; font-size: 12px;';
        indicator.textContent = 'Refactored Engine Active';
        document.body.appendChild(indicator);
        
        // Run migration test
        setTimeout(async () => {
            eventLogger.log('Running migration test...');
            const results = await adapter.runMigrationTest();
            eventLogger.log(`Migration test results: ${JSON.stringify(results)}`);
        }, 2000);
        
    } catch (error) {
        console.error('Failed to initialize refactored engine:', error);
        eventLogger.log(`Failed to initialize refactored engine: ${error.message}`);
    }
}

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    appInitializer.cleanup();
});

// Start the application when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', main);
} else {
    // DOM is already loaded
    main();
}