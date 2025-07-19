/**
 * Router Details UI Module
 * Manages the detailed router information display in the sidebar
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';

class RouterDetailsUI {
    constructor() {
        this.expandedRouters = new Set();
        this.activeTab = new Map(); // routerId -> activeTab
        this.updateInterval = null;
        this.isUpdating = false;
    }

    init() {
        // Listen for router list updates
        window.addEventListener('routerListUpdated', () => {
            this.updateAllExpandedRouters();
        });
        
        // Start periodic updates during simulation
        window.addEventListener('simulationStarted', () => {
            this.startPeriodicUpdates();
        });
        
        window.addEventListener('simulationStopped', () => {
            this.stopPeriodicUpdates();
        });
    }

    createRouterCard(router) {
        const isExpanded = this.expandedRouters.has(router.id);
        const statusBadges = this.createStatusBadges(router);
        const classes = this.getRouterClasses(router);
        
        return `
            <div class="${classes.join(' ')}" data-router-id="${router.id}">
                <div class="router-header">
                    <div class="router-header-clickable" data-router-id="${router.id}">
                        <div class="router-header-left">
                            <span class="expand-icon">${isExpanded ? '▼' : '▶'}</span>
                            <span class="router-name">${router.name} (ID: ${router.id})</span>
                        </div>
                        <div class="router-status">
                            ${statusBadges.join('')}
                        </div>
                    </div>
                    <button class="router-config-btn" data-router-id="${router.id}" title="Router Configuration">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="3"/>
                            <path d="M12 1v6m0 6v6m6.364-15.364l-4.243 4.243m-4.242 4.242l-4.243 4.243m20.364-6.364h-6m-6 0h-6m15.364 6.364l-4.243-4.243m-4.242-4.242l-4.243-4.243"/>
                        </svg>
                    </button>
                </div>
                <div class="router-content ${isExpanded ? 'expanded' : 'collapsed'}" id="router-content-${router.id}">
                    ${isExpanded ? this.createRouterDetailsContent(router.id) : ''}
                </div>
            </div>
        `;
    }

    createStatusBadges(router) {
        const badges = [];
        if (router.ospf_enabled) {
            badges.push('<span class="status-badge status-ospf">OSPF</span>');
        }
        if (router.is_failed) {
            badges.push('<span class="status-badge status-failed">FAILED</span>');
        }
        return badges;
    }

    getRouterClasses(router) {
        const classes = ['router-card'];
        if (router.ospf_enabled) classes.push('ospf-enabled');
        if (router.is_failed) classes.push('failed');
        if (this.expandedRouters.has(router.id)) classes.push('expanded');
        return classes;
    }

    toggleRouterDetails(routerId) {
        const content = document.getElementById(`router-content-${routerId}`);
        const card = document.querySelector(`[data-router-id="${routerId}"]`);
        const expandIcon = card.querySelector('.expand-icon');
        
        if (!content) return;
        
        if (this.expandedRouters.has(routerId)) {
            // Collapse
            this.expandedRouters.delete(routerId);
            content.classList.remove('expanded');
            content.classList.add('collapsed');
            expandIcon.textContent = '▶';
            setTimeout(() => {
                content.innerHTML = '';
            }, 300);
        } else {
            // Expand
            this.expandedRouters.add(routerId);
            content.innerHTML = this.createRouterDetailsContent(routerId);
            content.classList.remove('collapsed');
            content.classList.add('expanded');
            expandIcon.textContent = '▼';
        }
    }

    createRouterDetailsContent(routerId) {
        const activeTab = this.activeTab.get(routerId) || 'summary';
        
        return `
            <div class="router-tabs">
                <button class="tab-button ${activeTab === 'summary' ? 'active' : ''}" 
                        data-router-id="${routerId}" data-tab="summary">概要</button>
                <button class="tab-button ${activeTab === 'routes' ? 'active' : ''}" 
                        data-router-id="${routerId}" data-tab="routes">ルート</button>
                <button class="tab-button ${activeTab === 'lsa' ? 'active' : ''}" 
                        data-router-id="${routerId}" data-tab="lsa">LSA DB</button>
                <button class="tab-button ${activeTab === 'neighbors' ? 'active' : ''}" 
                        data-router-id="${routerId}" data-tab="neighbors">ネイバー</button>
            </div>
            <div class="tab-content" id="tab-content-${routerId}">
                ${this.getTabContent(routerId, activeTab)}
            </div>
        `;
    }

    switchTab(routerId, tab) {
        this.activeTab.set(routerId, tab);
        const content = document.getElementById(`tab-content-${routerId}`);
        if (content) {
            content.innerHTML = this.getTabContent(routerId, tab);
        }
        
        // Update tab buttons
        const card = document.querySelector(`[data-router-id="${routerId}"]`);
        const buttons = card.querySelectorAll('.tab-button');
        buttons.forEach(btn => {
            btn.classList.remove('active');
            if (btn.textContent === this.getTabLabel(tab)) {
                btn.classList.add('active');
            }
        });
    }

    getTabLabel(tab) {
        const labels = {
            'summary': '概要',
            'routes': 'ルート',
            'lsa': 'LSA DB',
            'neighbors': 'ネイバー'
        };
        return labels[tab] || tab;
    }

    getTabContent(routerId, tab) {
        try {
            switch (tab) {
                case 'summary':
                    return this.getSummaryContent(routerId);
                case 'routes':
                    return this.getRoutesContent(routerId);
                case 'lsa':
                    return this.getLSAContent(routerId);
                case 'neighbors':
                    return this.getNeighborsContent(routerId);
                default:
                    return '<div class="empty-state">データがありません</div>';
            }
        } catch (error) {
            console.error(`Error getting ${tab} content for router ${routerId}:`, error);
            return '<div class="error-state">データの取得に失敗しました</div>';
        }
    }

    getSummaryContent(routerId) {
        console.log('=== getSummaryContent called for router:', routerId);
        
        // デバッグ: simulator の状態を確認
        if (!stateManager.simulator) {
            console.error('stateManager.simulator is not initialized');
            return '<div class="error-state">シミュレータが初期化されていません</div>';
        }
        
        try {
            console.log('Calling get_router_summary_json...');
            const summaryJson = stateManager.simulator.get_router_summary_json(routerId);
            console.log('Calling get_router_details_json...');
            const detailsJson = stateManager.simulator.get_router_details_json(routerId);
            
            console.log('Router Summary JSON:', summaryJson);
            console.log('Router Details JSON:', detailsJson);
            
            if (!summaryJson || !detailsJson) {
                console.error('No data returned from simulator');
                return '<div class="empty-state">データがありません</div>';
            }
            
            const summary = JSON.parse(summaryJson);
            const details = JSON.parse(detailsJson);
            console.log('Parsed details:', details);
        
        const summaryHTML = `
            <div class="summary-content">
                <div class="summary-item">
                    <span class="label">Router ID:</span>
                    <span class="value">${details.id || details.router_id || routerId}</span>
                </div>
                <div class="summary-item">
                    <span class="label">OSPF状態:</span>
                    <span class="value ${summary.ospf_enabled ? 'active' : 'inactive'}">
                        ${summary.ospf_enabled ? '有効' : '無効'}
                    </span>
                </div>
                <div class="summary-item">
                    <span class="label">ネイバー数:</span>
                    <span class="value">${summary.neighbor_count || 0}</span>
                </div>
                <div class="summary-item">
                    <span class="label">ルート数:</span>
                    <span class="value">${summary.route_count || 0}</span>
                </div>
                <div class="summary-item">
                    <span class="label">LSAデータベースサイズ:</span>
                    <span class="value">${details.lsa_database_size || details.ospf_lsas || 0}</span>
                </div>
                <div class="summary-item">
                    <span class="label">最新イベント:</span>
                    <span class="value event">${summary.latest_event || 'なし'}</span>
                </div>
                ${details.interfaces ? this.createInterfacesSummary(details.interfaces, routerId) : ''}
            </div>
        `;
        console.log('=== Generated Summary HTML ===');
        console.log(summaryHTML);
        
        // DOMに実際に挿入された後の内容を確認
        setTimeout(() => {
            const ifElements = document.querySelectorAll('.interface-item .if-name');
            console.log('=== Actual DOM interface names ===');
            ifElements.forEach((el, index) => {
                console.log(`DOM element ${index} text: "${el.textContent}"`);
                console.log(`DOM element ${index} HTML: "${el.innerHTML}"`);
                console.log(`DOM element ${index} parent HTML:`, el.parentElement.outerHTML);
            });
            
            // 「if1」というテキストを含む要素を探す
            const allElements = document.querySelectorAll('*');
            const if1Elements = Array.from(allElements).filter(el => 
                el.textContent && el.textContent.includes('if1') && 
                !el.textContent.includes('IF1')
            );
            
            if (if1Elements.length > 0) {
                console.log('=== Found elements containing "if1" ===');
                if1Elements.forEach(el => {
                    console.log('Element:', el);
                    console.log('Tag:', el.tagName);
                    console.log('Class:', el.className);
                    console.log('Text:', el.textContent);
                    console.log('HTML:', el.outerHTML);
                });
            }
        }, 100);
        
        return summaryHTML;
        } catch (error) {
            console.error('Error in getSummaryContent:', error);
            return '<div class="error-state">データの処理中にエラーが発生しました</div>';
        }
    }

    createInterfacesSummary(interfaces, routerId) {
        // デバッグ: インターフェース情報を詳しくログ出力
        console.log('=== Interface Details Debug ===');
        interfaces.forEach(iface => {
            console.log(`Interface ${iface.id}:`, {
                name: iface.name,
                name_type: typeof iface.name,
                name_length: iface.name ? iface.name.length : 0,
                ip: iface.ip_address,
                cost: iface.cost,
                full_object: iface
            });
        });
        
        return `
            <div class="interfaces-summary">
                <h4>インターフェース</h4>
                ${interfaces.map(iface => {
                    // インターフェース名が存在し、空でない場合はそれを使用
                    const ifName = (iface.name && iface.name.trim() !== '') ? iface.name : `IF${iface.id}`;
                    console.log(`Interface ${iface.id} display name: "${ifName}" (original: "${iface.name}")`);
                    // 生成されるHTMLを確認
                    const html = `
                        <div class="interface-item">
                            <span class="if-name">${ifName}:</span>
                            <span class="if-ip">${iface.ip_address}</span>
                            <span class="if-cost">Cost: ${iface.cost}</span>
                            <button class="config-btn" onclick="window.routerDetailsUI.openInterfaceConfig(${routerId}, ${iface.id})" title="設定">⚙️</button>
                        </div>
                    `;
                    console.log(`Generated HTML for interface ${iface.id}:`, html);
                    return html;
                }).join('')}
            </div>
        `;
    }

    getRoutesContent(routerId) {
        if (!stateManager.simulator) {
            console.error('stateManager.simulator is not initialized');
            return '<div class="error-state">シミュレータが初期化されていません</div>';
        }
        
        try {
            const detailsJson = stateManager.simulator.get_router_details_json(routerId);
            console.log('Routes Details JSON:', detailsJson);
            
            if (!detailsJson) {
                return '<div class="empty-state">ルーティングテーブルが空です</div>';
            }
            
            const details = JSON.parse(detailsJson);
            const routes = details.routing_table || [];
        
        if (routes.length === 0) {
            return '<div class="empty-state">ルーティングテーブルが空です</div>';
        }
        
        return `
            <div class="routes-content">
                <table class="routes-table">
                    <thead>
                        <tr>
                            <th>宛先</th>
                            <th>ネクストホップ</th>
                            <th>コスト</th>
                            <th>インターフェース</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${routes.map(route => `
                            <tr>
                                <td>${route.destination}/${route.netmask || '24'}</td>
                                <td>${route.next_hop || 'Direct'}</td>
                                <td>${route.metric || route.cost || 0}</td>
                                <td>${route.interface_name || `IF${route.interface_id || route.interface || '-'}`}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
        `;
        } catch (error) {
            console.error('Error in getRoutesContent:', error);
            return '<div class="error-state">ルーティングテーブルの処理中にエラーが発生しました</div>';
        }
    }

    getLSAContent(routerId) {
        if (!stateManager.simulator) {
            console.error('stateManager.simulator is not initialized');
            return '<div class="error-state">シミュレータが初期化されていません</div>';
        }
        
        try {
            const detailsJson = stateManager.simulator.get_router_details_json(routerId);
            console.log('LSA Details JSON:', detailsJson);
            
            if (!detailsJson) {
                return '<div class="empty-state">LSAデータベースが空です</div>';
            }
            
            const details = JSON.parse(detailsJson);
            const lsaDb = details.lsa_database || [];
        
        if (lsaDb.length === 0) {
            return '<div class="empty-state">LSAデータベースが空です</div>';
        }
        
        // Group LSAs by type
        const groupedLSAs = this.groupLSAsByType(lsaDb);
        
        return `
            <div class="lsa-content">
                ${Object.entries(groupedLSAs).map(([type, lsas]) => `
                    <div class="lsa-type-group">
                        <h4>${this.getLSATypeName(type)} (${lsas.length})</h4>
                        ${lsas.map(lsa => this.createLSAItem(lsa)).join('')}
                    </div>
                `).join('')}
            </div>
        `;
        } catch (error) {
            console.error('Error in getLSAContent:', error);
            return '<div class="error-state">LSAデータベースの処理中にエラーが発生しました</div>';
        }
    }

    groupLSAsByType(lsaDb) {
        const grouped = {};
        lsaDb.forEach(lsa => {
            const type = lsa.lsa_type || 'Unknown';
            if (!grouped[type]) {
                grouped[type] = [];
            }
            grouped[type].push(lsa);
        });
        return grouped;
    }

    getLSATypeName(type) {
        const typeNames = {
            'RouterLSA': 'Router LSA',
            'NetworkLSA': 'Network LSA',
            'SummaryLSA': 'Summary LSA',
            'ASExternalLSA': 'AS External LSA'
        };
        return typeNames[type] || type;
    }

    createLSAItem(lsa) {
        return `
            <div class="lsa-item">
                <div class="lsa-header">
                    <span class="lsa-id">ID: ${lsa.link_state_id}</span>
                    <span class="lsa-seq">Seq: ${lsa.sequence_number}</span>
                    <span class="lsa-age">Age: ${lsa.age}s</span>
                    <span class="lsa-checksum">Checksum: ${lsa.checksum || 'N/A'}</span>
                </div>
                ${lsa.connected_routers ? `
                    <div class="lsa-details">
                        接続: ${lsa.connected_routers.join(', ')}
                    </div>
                ` : ''}
            </div>
        `;
    }

    getNeighborsContent(routerId) {
        if (!stateManager.simulator) {
            console.error('stateManager.simulator is not initialized');
            return '<div class="error-state">シミュレータが初期化されていません</div>';
        }
        
        try {
            const detailsJson = stateManager.simulator.get_router_details_json(routerId);
            console.log('Neighbors Details JSON:', detailsJson);
            
            if (!detailsJson) {
                return '<div class="empty-state">ネイバーがいません</div>';
            }
            
            const details = JSON.parse(detailsJson);
            const neighbors = details.neighbors || details.ospf_neighbors || [];
        
        if (neighbors.length === 0) {
            return '<div class="empty-state">ネイバーがいません</div>';
        }
        
        return `
            <div class="neighbors-content">
                ${neighbors.map(neighbor => `
                    <div class="neighbor-item">
                        <div class="neighbor-header">
                            <span class="neighbor-id">Router ${neighbor.router_id}</span>
                            <span class="neighbor-state state-${neighbor.state.toLowerCase()}">
                                ${neighbor.state}
                            </span>
                        </div>
                        <div class="neighbor-details">
                            <span>IP: ${neighbor.ip_address}</span>
                            <span>Priority: ${neighbor.priority}</span>
                            ${neighbor.is_dr ? '<span class="dr-badge">DR</span>' : ''}
                            ${neighbor.is_bdr ? '<span class="bdr-badge">BDR</span>' : ''}
                        </div>
                    </div>
                `).join('')}
            </div>
        `;
        } catch (error) {
            console.error('Error in getNeighborsContent:', error);
            return '<div class="error-state">ネイバー情報の処理中にエラーが発生しました</div>';
        }
    }

    updateAllExpandedRouters() {
        if (this.isUpdating) return;
        this.isUpdating = true;
        
        this.expandedRouters.forEach(routerId => {
            const activeTab = this.activeTab.get(routerId) || 'summary';
            const content = document.getElementById(`tab-content-${routerId}`);
            if (content) {
                content.innerHTML = this.getTabContent(routerId, activeTab);
            }
        });
        
        this.isUpdating = false;
    }

    startPeriodicUpdates() {
        this.stopPeriodicUpdates();
        this.updateInterval = setInterval(() => {
            this.updateAllExpandedRouters();
        }, 2000); // Update every 2 seconds
    }

    stopPeriodicUpdates() {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
            this.updateInterval = null;
        }
    }

    openInterfaceConfig(routerId, interfaceId) {
        const router = stateManager.routers.get(routerId);
        if (!router) return;
        
        const iface = router.interfaces?.find(i => i.id === interfaceId);
        if (!iface) return;

        // ダイアログ作成
        const dialog = document.createElement('div');
        dialog.className = 'interface-config-dialog';
        dialog.innerHTML = `
            <div class="dialog-overlay" onclick="window.routerDetailsUI.closeInterfaceConfig()"></div>
            <div class="dialog-content">
                <h3>インターフェース設定 - ${iface.name || `IF${iface.id}`}</h3>
                <form id="interface-config-form">
                    <div class="form-group">
                        <label>IPアドレス:</label>
                        <input type="text" id="if-ip" value="${iface.ip_address}" pattern="^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$" required>
                    </div>
                    <div class="form-group">
                        <label>サブネットマスク:</label>
                        <input type="text" id="if-netmask" value="${iface.netmask}" pattern="^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$" required>
                    </div>
                    <div class="form-group">
                        <label>コスト:</label>
                        <input type="number" id="if-cost" value="${iface.cost}" min="1" max="65535" required>
                    </div>
                    <div class="form-group">
                        <label>Hello間隔 (秒):</label>
                        <input type="number" id="if-hello" value="${iface.hello_interval || 10}" min="1" max="65535" required>
                    </div>
                    <div class="form-group">
                        <label>Dead間隔 (秒):</label>
                        <input type="number" id="if-dead" value="${iface.dead_interval || 40}" min="1" max="65535" required>
                    </div>
                    <div class="form-group">
                        <label>優先度:</label>
                        <input type="number" id="if-priority" value="${iface.priority || 1}" min="0" max="255" required>
                    </div>
                    <div class="form-group">
                        <label>MTU:</label>
                        <input type="number" id="if-mtu" value="${iface.mtu || 1500}" min="576" max="9000" required>
                    </div>
                    <div class="form-group">
                        <label>
                            <input type="checkbox" id="if-enabled" ${iface.enabled ? 'checked' : ''}>
                            有効
                        </label>
                    </div>
                    
                    <h4>OSPF認証設定</h4>
                    <div class="form-group">
                        <label>認証タイプ:</label>
                        <select id="if-auth-type" onchange="window.routerDetailsUI.updateAuthFields()">
                            <option value="0" ${(!iface.auth_config || iface.auth_config.auth_type === 0) ? 'selected' : ''}>なし</option>
                            <option value="1" ${(iface.auth_config && iface.auth_config.auth_type === 1) ? 'selected' : ''}>シンプルパスワード</option>
                            <option value="2" ${(iface.auth_config && iface.auth_config.auth_type === 2) ? 'selected' : ''}>MD5</option>
                        </select>
                    </div>
                    <div id="auth-password-group" class="form-group" style="display: ${(iface.auth_config && iface.auth_config.auth_type > 0) ? 'block' : 'none'}">
                        <label>認証キー/パスワード:</label>
                        <input type="text" id="if-auth-key" value="${iface.auth_config?.auth_key || ''}" maxlength="8" placeholder="最大8文字">
                    </div>
                    <div id="auth-keyid-group" class="form-group" style="display: ${(iface.auth_config && iface.auth_config.auth_type === 2) ? 'block' : 'none'}">
                        <label>キーID (MD5):</label>
                        <input type="number" id="if-auth-keyid" value="${iface.auth_config?.key_id || 1}" min="1" max="255">
                    </div>
                    
                    <div class="form-buttons">
                        <button type="button" onclick="window.routerDetailsUI.saveInterfaceConfig(${routerId}, ${interfaceId})">保存</button>
                        <button type="button" onclick="window.routerDetailsUI.closeInterfaceConfig()">キャンセル</button>
                    </div>
                </form>
            </div>
        `;
        
        document.body.appendChild(dialog);
    }

    closeInterfaceConfig() {
        const dialog = document.querySelector('.interface-config-dialog');
        if (dialog) {
            dialog.remove();
        }
    }

    updateAuthFields() {
        const authType = parseInt(document.getElementById('if-auth-type').value);
        const passwordGroup = document.getElementById('auth-password-group');
        const keyIdGroup = document.getElementById('auth-keyid-group');
        
        if (authType === 0) {
            // No authentication
            passwordGroup.style.display = 'none';
            keyIdGroup.style.display = 'none';
        } else if (authType === 1) {
            // Simple password
            passwordGroup.style.display = 'block';
            keyIdGroup.style.display = 'none';
        } else if (authType === 2) {
            // MD5
            passwordGroup.style.display = 'block';
            keyIdGroup.style.display = 'block';
        }
    }

    async saveInterfaceConfig(routerId, interfaceId) {
        const authType = parseInt(document.getElementById('if-auth-type').value);
        const config = {
            ip_address: document.getElementById('if-ip').value,
            netmask: document.getElementById('if-netmask').value,
            cost: parseInt(document.getElementById('if-cost').value),
            hello_interval: parseInt(document.getElementById('if-hello').value),
            dead_interval: parseInt(document.getElementById('if-dead').value),
            priority: parseInt(document.getElementById('if-priority').value),
            mtu: parseInt(document.getElementById('if-mtu').value),
            enabled: document.getElementById('if-enabled').checked,
            auth_type: authType,
            auth_key: authType > 0 ? document.getElementById('if-auth-key').value : null,
            auth_key_id: authType === 2 ? parseInt(document.getElementById('if-auth-keyid').value) : null
        };

        try {
            stateManager.simulator.update_interface_config(routerId, interfaceId, JSON.stringify(config));
            
            // 成功したらダイアログを閉じて更新
            this.closeInterfaceConfig();
            this.updateAllExpandedRouters();
            
            eventLogger.addLogEntry('info', `Router ${routerId} Interface ${interfaceId} 設定を更新しました`);
        } catch (error) {
            console.error('Failed to update interface config:', error);
            alert('インターフェース設定の更新に失敗しました: ' + error.message);
        }
    }

    openRouterConfig(routerId) {
        console.log(`Opening router config for router ${routerId}`);
        
        const router = stateManager.routers.find(r => r.id === routerId);
        if (!router) return;
        
        this.showRouterConfigDialog(router);
    }

    showRouterConfigDialog(router) {
        // Remove existing dialog if any
        const existingDialog = document.getElementById('router-config-dialog');
        if (existingDialog) {
            existingDialog.remove();
        }
        
        const dialog = document.createElement('div');
        dialog.id = 'router-config-dialog';
        dialog.className = 'config-dialog-overlay';
        dialog.innerHTML = `
            <div class="config-dialog">
                <div class="dialog-header">
                    <h3>Router Configuration - ${router.name}</h3>
                    <button class="close-btn" onclick="window.routerDetailsUI.closeRouterConfig()">×</button>
                </div>
                <div class="dialog-body">
                    <form id="router-config-form">
                        <div class="form-section">
                            <h4>Basic Settings</h4>
                            <div class="form-group">
                                <label>Router Name:</label>
                                <input type="text" id="router-name" value="${router.name}" required>
                            </div>
                        </div>
                        
                        <div class="form-section">
                            <h4>OSPF Settings</h4>
                            <div class="form-group">
                                <label>
                                    <input type="checkbox" id="ospf-enabled" ${router.ospf_enabled ? 'checked' : ''}>
                                    Enable OSPF
                                </label>
                            </div>
                            <div id="ospf-params" style="display: ${router.ospf_enabled ? 'block' : 'none'}">
                                <div class="form-group">
                                    <label>Router ID:</label>
                                    <input type="text" id="router-id" value="${router.id}.${router.id}.${router.id}.${router.id}" pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$">
                                    <small>Format: x.x.x.x (e.g., 1.1.1.1)</small>
                                </div>
                                <div class="form-group">
                                    <label>Area ID:</label>
                                    <input type="text" id="area-id" value="0.0.0.0" pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$">
                                    <small>Format: x.x.x.x (e.g., 0.0.0.0 for backbone)</small>
                                </div>
                            </div>
                        </div>
                        
                        <div class="form-actions">
                            <button type="submit" class="btn-primary">Apply</button>
                            <button type="button" class="btn-secondary" onclick="window.routerDetailsUI.closeRouterConfig()">Cancel</button>
                        </div>
                    </form>
                </div>
            </div>
        `;
        
        document.body.appendChild(dialog);
        
        // Handle OSPF enable/disable toggle
        const ospfCheckbox = document.getElementById('ospf-enabled');
        const ospfParams = document.getElementById('ospf-params');
        ospfCheckbox.addEventListener('change', (e) => {
            ospfParams.style.display = e.target.checked ? 'block' : 'none';
        });
        
        // Handle form submission
        const form = document.getElementById('router-config-form');
        form.addEventListener('submit', (e) => {
            e.preventDefault();
            this.applyRouterConfig(router.id);
        });
    }

    closeRouterConfig() {
        const dialog = document.getElementById('router-config-dialog');
        if (dialog) {
            dialog.remove();
        }
    }

    applyRouterConfig(routerId) {
        const name = document.getElementById('router-name').value;
        const ospfEnabled = document.getElementById('ospf-enabled').checked;
        
        // Note: Actual router name change and OSPF enable/disable would need
        // backend implementation. For now, we'll just handle OSPF enable/disable
        
        if (!stateManager.simulator) return;
        
        try {
            // Toggle OSPF if state changed
            const router = stateManager.routers.find(r => r.id === routerId);
            if (router && router.ospf_enabled !== ospfEnabled) {
                if (ospfEnabled) {
                    stateManager.simulator.enable_ospf(routerId);
                    eventLogger.log(`OSPF enabled on router ${name}`);
                } else {
                    // Note: disable_ospf method would need to be implemented
                    eventLogger.log(`OSPF disable requested for router ${name} (not implemented)`);
                }
            }
            
            // Close dialog
            this.closeRouterConfig();
            
            // Refresh router list
            window.dispatchEvent(new Event('routersUpdated'));
            
        } catch (error) {
            console.error('Failed to update router configuration:', error);
            alert('Failed to update router configuration: ' + error);
        }
    }
}

// Export singleton instance
const routerDetailsUI = new RouterDetailsUI();
window.routerDetailsUI = routerDetailsUI; // Make available globally for onclick handlers
export default routerDetailsUI;