// Test script for real-time updates
// Run this in browser console to verify updates are working

console.log('=== Real-time Update Test ===');

// Check if router details UI is initialized
if (window.routerDetailsUI) {
    console.log('✓ RouterDetailsUI is initialized');
    console.log('  Update interval ID:', window.routerDetailsUI.updateInterval);
    console.log('  Expanded routers:', Array.from(window.routerDetailsUI.expandedRouters));
} else {
    console.error('✗ RouterDetailsUI is NOT initialized');
}

// Check if app initializer periodic updates are running
if (window.appInitializer && window.appInitializer.updateInterval) {
    console.log('✓ App initializer periodic updates are running');
    console.log('  Update interval ID:', window.appInitializer.updateInterval);
} else {
    console.error('✗ App initializer periodic updates are NOT running');
}

// Manually trigger an update
console.log('\nManually triggering update...');
if (window.routerDetailsUI) {
    window.routerDetailsUI.updateAllExpandedRouters();
}

// Monitor updates for 10 seconds
console.log('\nMonitoring updates for 10 seconds...');
let updateCount = 0;
const originalUpdate = window.routerDetailsUI ? window.routerDetailsUI.updateAllExpandedRouters.bind(window.routerDetailsUI) : null;

if (window.routerDetailsUI && originalUpdate) {
    window.routerDetailsUI.updateAllExpandedRouters = function() {
        updateCount++;
        console.log(`[Monitor] Update #${updateCount} triggered at`, new Date().toISOString());
        return originalUpdate();
    };
    
    setTimeout(() => {
        window.routerDetailsUI.updateAllExpandedRouters = originalUpdate;
        console.log(`\n=== Test Complete ===`);
        console.log(`Total updates in 10 seconds: ${updateCount}`);
        console.log('Expected: ~10 updates (1 per second)');
    }, 10000);
}