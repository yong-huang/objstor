class ObjStorApp {
    constructor() {
        this.ws = null;
        this.currentPage = 'dashboard';
        this.charts = {};
        this.startTime = Date.now();
    }

    async init() {
        this.setupNavigation();
        this.connectWebSocket();
        await this.loadPage('dashboard');
        this.updateUptime();
        setInterval(() => this.updateUptime(), 1000);
    }

    setupNavigation() {
        document.querySelectorAll('.nav-item').forEach(item => {
            item.addEventListener('click', () => {
                const page = item.dataset.page;
                if (page) {
                    this.loadPage(page);
                }
            });
        });
    }

    async loadPage(page) {
        // Update nav state
        document.querySelectorAll('.nav-item').forEach(item => {
            item.classList.remove('active');
            if (item.dataset.page === page) {
                item.classList.add('active');
            }
        });

        // Show page content
        document.querySelectorAll('.page-content').forEach(content => {
            content.classList.remove('active');
        });
        const pageContent = document.getElementById(`page-${page}`);
        if (pageContent) {
            pageContent.classList.add('active');
        }

        this.currentPage = page;

        // Load page-specific data
        switch(page) {
            case 'dashboard':
                await this.loadDashboard();
                break;
            case 'buckets':
                await this.loadBuckets();
                break;
            case 'objects':
                await this.loadObjects();
                break;
            case 'monitoring':
                await this.loadMonitoring();
                break;
            case 'logs':
                this.initLogs();
                break;
        }
    }

    connectWebSocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

        ws.onopen = () => {
            console.log('WebSocket connected');
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                this.handleWebSocketMessage(data);
            } catch (e) {
                console.error('Failed to parse WebSocket message:', e);
            }
        };

        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };

        ws.onclose = () => {
            console.log('WebSocket closed, reconnecting...');
            setTimeout(() => this.connectWebSocket(), 3000);
        };

        this.ws = ws;
    }

    handleWebSocketMessage(data) {
        switch(data.type) {
            case 'connected':
                console.log('Connected to ObjStor:', data.message);
                break;
            case 'metrics':
                if (this.currentPage === 'dashboard' || this.currentPage === 'monitoring') {
                    this.updateMetrics(data.data);
                    if (this.currentPage === 'dashboard') {
                        this.updateStorageChart(data.data);
                    }
                    if (this.currentPage === 'monitoring') {
                        this.updateMonitoringChart(data.data);
                    }
                }
                break;
            case 'log':
                if (this.currentPage === 'logs') {
                    this.appendLog(data.data);
                }
                break;
            default:
                console.log('Unknown message type:', data.type);
        }
    }

    async loadDashboard() {
        try {
            const response = await fetch('/api/v1/metrics');
            const data = await response.json();
            this.updateMetrics(data);
            this.initStorageChart(data);
        } catch (error) {
            console.error('Failed to load dashboard:', error);
        }
    }

    updateMetrics(data) {
        if (!data) return;

        console.log('updateMetrics received:', data);

        // Update summary cards
        const storage = data.storage || {};
        const used = this.formatBytes(storage.used || 0);
        const capacity = this.formatBytes(storage.capacity || 0);

        document.getElementById('total-storage').textContent = `${used} / ${capacity}`;

        const bucketCount = data.buckets?.length || 0;
        console.log('Bucket count:', bucketCount, data.buckets);
        document.getElementById('bucket-count').textContent = bucketCount;

        // Use total_objects from database instead of summing pool objects
        const objectCount = data.total_objects || 0;
        console.log('Total object count from DB:', objectCount);
        document.getElementById('object-count').textContent = objectCount.toLocaleString();

        // Update pools list
        const pools = data.pools || [];
        this.updatePoolsList(pools);
    }

    updatePoolsList(pools) {
        const container = document.getElementById('pools-list');
        if (!container) return;

        if (pools.length === 0) {
            container.innerHTML = '<p class="text-gray">No storage pools configured.</p>';
            return;
        }

        container.innerHTML = pools.map(pool => {
            const usagePercent = ((pool.used / pool.capacity) * 100).toFixed(1);
            return `
                <div class="pool-item">
                    <div class="pool-info w-full">
                        <h3>${pool.id}</h3>
                        <div class="pool-meta">
                            <span>${this.formatBytes(pool.used)} / ${this.formatBytes(pool.capacity)}</span>
                            <span>${pool.objects} objects</span>
                            <span>${usagePercent}% used</span>
                        </div>
                        <div class="pool-usage-bar">
                            <div class="pool-usage-fill" style="width: ${usagePercent}%"></div>
                        </div>
                    </div>
                </div>
            `;
        }).join('');
    }

    initStorageChart(metricsData) {
        const ctx = document.getElementById('storage-chart');
        if (!ctx) return;

        if (this.charts.storage) {
            this.charts.storage.destroy();
        }

        const storageUsedGB = (metricsData?.storage?.used || 0) / (1024 * 1024 * 1024);
        const now = new Date().toLocaleTimeString();

        this.charts.storage = new Chart(ctx, {
            type: 'line',
            data: {
                labels: [now],
                datasets: [{
                    label: 'Storage Usage (GB)',
                    data: [storageUsedGB.toFixed(2)],
                    borderColor: '#3b82f6',
                    backgroundColor: 'rgba(59, 130, 246, 0.1)',
                    fill: true,
                    tension: 0.4
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    y: {
                        beginAtZero: true,
                        title: {
                            display: true,
                            text: 'Size (GB)'
                        }
                    }
                },
                plugins: {
                    legend: {
                        display: false
                    }
                }
            }
        });
    }

    updateStorageChart(metricsData) {
        if (!this.charts.storage) return;

        const chart = this.charts.storage;
        const storageUsedGB = (metricsData?.storage?.used || 0) / (1024 * 1024 * 1024);
        const now = new Date().toLocaleTimeString();

        // Add new data point
        chart.data.labels.push(now);
        chart.data.datasets[0].data.push(storageUsedGB.toFixed(2));

        // Keep only last 20 data points
        if (chart.data.labels.length > 20) {
            chart.data.labels.shift();
            chart.data.datasets[0].data.shift();
        }

        chart.update('none');
    }

    async loadBuckets() {
        try {
            const response = await fetch('/api/v1/buckets');
            const data = await response.json();
            this.updateBucketsList(data.buckets || []);
        } catch (error) {
            console.error('Failed to load buckets:', error);
        }
    }

    updateBucketsList(buckets) {
        const container = document.getElementById('buckets-list');
        if (!container) return;

        if (buckets.length === 0) {
            container.innerHTML = '<p class="text-gray">No buckets found.</p>';
            return;
        }

        container.innerHTML = buckets.map(bucket => `
            <div class="bucket-item">
                <div class="bucket-info">
                    <h3>${bucket.name}</h3>
                    <div class="bucket-meta">
                        <span>Created: ${new Date(bucket.created_at).toLocaleDateString()}</span>
                        <span>Region: ${bucket.region}</span>
                        ${bucket.preferred_pool ? `<span>Pool: <strong>${bucket.preferred_pool}</strong></span>` : '<span>Pool: <em>Auto-select</em></span>'}
                    </div>
                </div>
                <button class="btn btn-light" onclick="deleteBucket('${bucket.name}')">
                    <svg viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
                    Delete
                </button>
            </div>
        `).join('');
    }

    async loadObjects() {
        // Load buckets for the selector
        try {
            const response = await fetch('/api/v1/buckets');
            const data = await response.json();
            await this.updateBucketSelector(data.buckets || []);
        } catch (error) {
            console.error('Failed to load buckets:', error);
        }

        // If a bucket is selected, load its objects
        const selectedBucket = document.getElementById('bucket-selector')?.value;
        if (selectedBucket) {
            await this.loadObjectsList(selectedBucket);
        }
    }

    async updateBucketSelector(buckets) {
        const selector = document.getElementById('bucket-selector');
        if (!selector) return;

        if (buckets.length === 0) {
            selector.innerHTML = '<option value="">No buckets available</option>';
            return;
        }

        selector.innerHTML = buckets.map(bucket =>
            `<option value="${bucket.name}">${bucket.name}</option>`
        ).join('');

        // Add change event listener
        selector.onchange = async () => {
            const bucket = selector.value;
            if (bucket) {
                await this.loadObjectsList(bucket);
            } else {
                document.getElementById('objects-list').innerHTML = '<p class="text-gray">Select a bucket to view objects.</p>';
            }
        };

        // Auto-select first bucket
        if (buckets.length > 0) {
            await this.loadObjectsList(buckets[0].name);
        }
    }

    async loadObjectsList(bucket) {
        try {
            // Use S3 API to list objects
            const response = await fetch(`/${encodeURIComponent(bucket)}?list-type=2`);
            if (!response.ok) {
                throw new Error(`Failed to list objects: ${response.status}`);
            }

            const xmlText = await response.text();
            const objects = this.parseS3ListObjectsResponse(xmlText);
            this.updateObjectsList(objects, bucket);
        } catch (error) {
            console.error('Failed to load objects:', error);
            const container = document.getElementById('objects-list');
            if (container) {
                container.innerHTML = `<p class="text-gray">Error loading objects: ${error.message}</p>`;
            }
        }
    }

    parseS3ListObjectsResponse(xmlText) {
        const parser = new DOMParser();
        const xmlDoc = parser.parseFromString(xmlText, "text/xml");
        const contents = xmlDoc.getElementsByTagName("Contents");
        const objects = [];

        for (let i = 0; i < contents.length; i++) {
            const item = contents[i];
            objects.push({
                key: item.getElementsByTagName("Key")[0]?.textContent || '',
                size: parseInt(item.getElementsByTagName("Size")[0]?.textContent || '0'),
                lastModified: item.getElementsByTagName("LastModified")[0]?.textContent || '',
                etag: item.getElementsByTagName("ETag")[0]?.textContent || ''
            });
        }

        return objects;
    }

    updateObjectsList(objects, bucket) {
        const container = document.getElementById('objects-list');
        if (!container) return;

        if (objects.length === 0) {
            container.innerHTML = '<p class="text-gray">No objects found in this bucket.</p>';
            return;
        }

        container.innerHTML = `
            <div class="objects-header">
                <span>Bucket: <strong>${bucket}</strong></span>
                <span>${objects.length} objects</span>
            </div>
            <div class="objects-list">
                ${objects.map(obj => `
                    <div class="object-item">
                        <div class="object-info">
                            <h3>${obj.key}</h3>
                            <div class="object-meta">
                                <span>${this.formatBytes(obj.size)}</span>
                                <span>${new Date(obj.lastModified).toLocaleString()}</span>
                            </div>
                        </div>
                        <div class="object-actions">
                            <button class="btn btn-light" onclick="downloadObject('${bucket}', '${obj.key}')">
                                <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                                Download
                            </button>
                            <button class="btn btn-light" onclick="deleteObject('${bucket}', '${obj.key}')">
                                <svg viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
                                Delete
                            </button>
                        </div>
                    </div>
                `).join('')}
            </div>
        `;
    }

    async loadMonitoring() {
        const ctx = document.getElementById('metrics-chart');
        if (!ctx) return;

        if (this.charts.metrics) {
            this.charts.metrics.destroy();
        }

        this.charts.metrics = new Chart(ctx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: 'Storage Usage (%)',
                    data: [],
                    borderColor: '#10b981',
                    backgroundColor: 'rgba(16, 185, 129, 0.1)',
                    fill: true,
                    tension: 0.4
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    y: {
                        beginAtZero: true,
                        max: 100
                    }
                },
                plugins: {
                    legend: {
                        display: true
                    }
                }
            }
        });

        // Load initial metrics
        try {
            const response = await fetch('/api/v1/metrics');
            const data = await response.json();
            this.updateMonitoringChart(data);
        } catch (error) {
            console.error('Failed to load monitoring data:', error);
        }
    }

    updateMonitoringChart(metricsData) {
        if (!this.charts.metrics) return;

        const chart = this.charts.metrics;
        const now = new Date().toLocaleTimeString();

        // Add new data point
        chart.data.labels.push(now);
        chart.data.datasets[0].data.push((metricsData.storage?.usage_ratio * 100).toFixed(2));

        // Keep only last 20 data points
        if (chart.data.labels.length > 20) {
            chart.data.labels.shift();
            chart.data.datasets[0].data.shift();
        }

        chart.update('none'); // Update without animation for performance

        // Update summary cards
        const storageEl = document.getElementById('monitor-storage');
        const objectsEl = document.getElementById('monitor-objects');
        const poolsEl = document.getElementById('monitor-pools');
        const timeEl = document.getElementById('monitor-time');

        if (storageEl) {
            const usageRatio = (metricsData.storage?.usage_ratio * 100 || 0).toFixed(1);
            storageEl.textContent = `${usageRatio}%`;
        }

        if (objectsEl) {
            const totalObjects = metricsData.total_objects || 0;
            objectsEl.textContent = totalObjects.toLocaleString();
        }

        if (poolsEl) {
            const activePools = metricsData.pools?.filter(p => p.status === 'Healthy').length || 0;
            const totalPools = metricsData.pools?.length || 0;
            poolsEl.textContent = `${activePools}/${totalPools}`;
        }

        if (timeEl) {
            timeEl.textContent = now;
        }
    }

    updateUptime() {
        const uptime = Date.now() - this.startTime;
        const seconds = Math.floor(uptime / 1000);
        const minutes = Math.floor(seconds / 60);
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);

        let uptimeText = '';
        if (days > 0) {
            uptimeText = `${days}d ${hours % 24}h ${minutes % 60}m`;
        } else if (hours > 0) {
            uptimeText = `${hours}h ${minutes % 60}m`;
        } else if (minutes > 0) {
            uptimeText = `${minutes}m ${seconds % 60}s`;
        } else {
            uptimeText = `${seconds}s`;
        }

        const uptimeEl = document.getElementById('uptime');
        if (uptimeEl) {
            uptimeEl.textContent = uptimeText;
        }
    }

    formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    initLogs() {
        const container = document.getElementById('logs-container');
        if (container) {
            container.innerHTML = '<div class="log-entry"><span class="log-timestamp">[System]</span> <span class="log-level-info">Connected to logs stream</span></div>';
        }
    }

    appendLog(logData) {
        const container = document.getElementById('logs-container');
        if (!container) return;

        const timestamp = new Date(logData.timestamp || Date.now()).toLocaleTimeString();
        const level = logData.level || 'info';
        const levelClass = `log-level-${level.toLowerCase()}`;
        const message = logData.message || logData.toString();

        const logEntry = document.createElement('div');
        logEntry.className = 'log-entry';
        logEntry.innerHTML = `
            <span class="log-timestamp">[${timestamp}]</span>
            <span class="${levelClass}">${level.toUpperCase()}</span>
            <span>${message}</span>
        `;

        container.appendChild(logEntry);

        // Auto-scroll to bottom
        container.scrollTop = container.scrollHeight;

        // Limit log entries to prevent memory issues
        while (container.children.length > 100) {
            container.removeChild(container.firstChild);
        }
    }

    showToast(type, title, message, duration = 4000) {
        const container = document.getElementById('toast-container');
        if (!container) return;

        const icons = {
            success: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>',
            error: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>',
            warning: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
            info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>'
        };

        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.innerHTML = `
            <div class="toast-icon">${icons[type]}</div>
            <div class="toast-content">
                <div class="toast-title">${title}</div>
                ${message ? `<div class="toast-message">${message}</div>` : ''}
            </div>
            <div class="toast-close" onclick="this.parentElement.remove()">
                <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="18" y1="6" x2="6" y2="18"/>
                    <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
            </div>
        `;

        container.appendChild(toast);

        // Auto-remove after duration
        setTimeout(() => {
            toast.classList.add('removing');
            setTimeout(() => toast.remove(), 300);
        }, duration);
    }

    showModal(options) {
        return new Promise((resolve) => {
            const overlay = document.getElementById('modal-overlay');
            const titleEl = document.getElementById('modal-title');
            const bodyEl = document.getElementById('modal-body');
            const footerEl = document.getElementById('modal-footer');

            if (!overlay || !titleEl || !bodyEl || !footerEl) {
                resolve(null);
                return;
            }

            // Set title
            titleEl.textContent = options.title || '';

            // Set body content
            if (options.type === 'prompt') {
                bodyEl.innerHTML = `
                    <p class="modal-description">${options.message || ''}</p>
                    <label class="modal-label">${options.label || 'Enter value:'}</label>
                    <input type="text" class="modal-input" id="modal-input" value="${options.defaultValue || ''}" placeholder="${options.placeholder || ''}">
                `;
            } else if (options.type === 'confirm') {
                bodyEl.innerHTML = `
                    <p class="modal-description">${options.message || ''}</p>
                    ${options.warning ? `<p class="modal-warning-text">⚠️ ${options.warning}</p>` : ''}
                `;
            } else if (options.type === 'custom') {
                bodyEl.innerHTML = options.html || '';
            }

            // Set footer buttons
            footerEl.innerHTML = `
                <button class="btn btn-light" onclick="app.closeModal(false)">${options.cancelText || 'Cancel'}</button>
                <button class="btn ${options.danger ? 'btn-danger' : 'btn-primary'}" id="modal-confirm">${options.confirmText || 'Confirm'}</button>
            `;

            // Show modal
            overlay.style.display = 'flex';

            // Handle confirm button
            const confirmBtn = document.getElementById('modal-confirm');
            if (confirmBtn) {
                confirmBtn.onclick = () => {
                    if (options.type === 'prompt') {
                        const input = document.getElementById('modal-input');
                        const value = input ? input.value : '';
                        this.closeModal(true, value);
                        resolve(value);
                    } else if (options.type === 'custom' && options.onConfirm) {
                        const result = options.onConfirm();
                        this.closeModal(true);
                        resolve(result);
                    } else {
                        this.closeModal(true);
                        resolve(true);
                    }
                };
            }

            // Handle escape key
            const handleEscape = (e) => {
                if (e.key === 'Escape') {
                    this.closeModal(false);
                    resolve(options.type === 'prompt' ? null : false);
                    document.removeEventListener('keydown', handleEscape);
                }
            };
            document.addEventListener('keydown', handleEscape);

            // Store handler for cleanup
            overlay._escapeHandler = handleEscape;
        });
    }

    closeModal(result = null, value = null) {
        const overlay = document.getElementById('modal-overlay');
        if (overlay) {
            overlay.style.display = 'none';

            // Clean up event listener
            if (overlay._escapeHandler) {
                document.removeEventListener('keydown', overlay._escapeHandler);
                delete overlay._escapeHandler;
            }
        }
        return result;
    }
}

// Global functions
async function createBucket() {
    // Fetch available pools first
    let pools = [];
    try {
        const metricsResponse = await fetch('/api/v1/metrics');
        const metricsData = await metricsResponse.json();
        pools = metricsData.pools || [];
    } catch (error) {
        console.error('Failed to fetch pools:', error);
    }

    // Build pool options HTML
    const poolOptions = pools.map(pool =>
        `<option value="${pool.id}">${pool.id} (${(pool.used / 1024 / 1024 / 1024).toFixed(2)} GB used, ${(pool.usage_ratio * 100).toFixed(1)}%)</option>`
    ).join('');

    const result = await app.showModal({
        type: 'custom',
        title: 'Create Bucket',
        html: `
            <p class="modal-description">Enter a name for the new bucket and optionally select a storage pool.</p>
            <label class="modal-label">Bucket Name:</label>
            <input type="text" class="modal-input" id="modal-bucket-name" placeholder="my-bucket" style="margin-bottom: 1rem;">

            <details style="margin-bottom: 1rem;">
                <summary style="cursor: pointer; color: #374151; font-weight: 500; padding: 0.5rem 0;">
                    Advanced: Storage Pool Selection
                </summary>
                <div style="margin-top: 0.5rem;">
                    <label class="modal-label">Storage Pool (optional):</label>
                    <select class="modal-input" id="modal-pool-select">
                        <option value="">Auto-select (Recommended)</option>
                        ${poolOptions}
                    </select>
                    <p class="modal-description" style="margin-top: 0.5rem; font-size: 0.8rem;">
                        Leave empty to let the system automatically select the best pool based on available space.
                    </p>
                </div>
            </details>
        `,
        confirmText: 'Create',
        cancelText: 'Cancel',
        onConfirm: () => {
            const nameInput = document.getElementById('modal-bucket-name');
            const poolSelect = document.getElementById('modal-pool-select');
            return {
                name: nameInput ? nameInput.value : '',
                poolId: poolSelect ? poolSelect.value : ''
            };
        }
    });

    if (!result || !result.name) return;

    const { name, poolId } = result;

    try {
        const headers = {
            'Content-Type': 'application/xml'
        };

        // Add pool header if specified
        if (poolId) {
            headers['x-amz-bucket-pool'] = poolId;
        }

        const response = await fetch(`/${encodeURIComponent(name)}`, {
            method: 'PUT',
            headers: headers,
            body: `<?xml version="1.0" encoding="UTF-8"?><CreateBucketConfiguration><LocationConstraint>us-east-1</LocationConstraint></CreateBucketConfiguration>`
        });

        if (response.ok) {
            const poolMsg = poolId ? ` (on pool ${poolId})` : '';
            app.showToast('success', 'Bucket Created', `Bucket "${name}" has been created successfully${poolMsg}.`);
            app.loadBuckets();
        } else {
            const status = response.status;
            let errorMessage = `Failed to create bucket`;

            if (status === 409) {
                errorMessage = `Bucket "${name}" already exists.`;
            } else if (status === 400) {
                errorMessage = `Invalid bucket name "${name}". Bucket names must be DNS-compliant.`;
            } else if (status === 404) {
                errorMessage = `Pool "${poolId}" not found.`;
            } else {
                const errorText = await response.text();
                errorMessage = errorText || `Failed to create bucket: ${status}`;
            }

            app.showToast('error', 'Creation Failed', errorMessage);
        }
    } catch (error) {
        app.showToast('error', 'Error', error.message);
    }
}

async function deleteBucket(name) {
    const confirmed = await app.showModal({
        type: 'confirm',
        title: 'Delete Bucket',
        message: `Are you sure you want to delete the bucket "${name}"?`,
        warning: 'This action cannot be undone.',
        confirmText: 'Delete',
        cancelText: 'Cancel',
        danger: true
    });

    if (!confirmed) return;

    try {
        const response = await fetch(`/${encodeURIComponent(name)}`, {
            method: 'DELETE'
        });

        if (response.ok) {
            app.showToast('success', 'Bucket Deleted', `Bucket "${name}" has been deleted successfully.`);
            app.loadBuckets();
        } else {
            const status = response.status;
            let errorMessage = `Failed to delete bucket`;

            if (status === 409) {
                errorMessage = `Bucket "${name}" is not empty. Please delete all objects first.`;
            } else if (status === 404) {
                errorMessage = `Bucket "${name}" does not exist.`;
            } else {
                const errorText = await response.text();
                errorMessage = errorText || `Failed to delete bucket: ${status}`;
            }

            app.showToast('error', 'Deletion Failed', errorMessage);
        }
    } catch (error) {
        app.showToast('error', 'Error', error.message);
    }
}

async function uploadObject() {
    const bucket = document.getElementById('bucket-selector')?.value;
    if (!bucket) {
        app.showToast('warning', 'No Bucket Selected', 'Please select a bucket first!');
        return;
    }

    const input = document.createElement('input');
    input.type = 'file';
    input.onchange = async (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const key = await app.showModal({
            type: 'prompt',
            title: 'Upload Object',
            message: `Selected file: ${file.name}`,
            label: 'Object Key (name):',
            defaultValue: file.name,
            placeholder: file.name,
            confirmText: 'Upload',
            cancelText: 'Cancel'
        });
        if (!key) return;

        try {
            const response = await fetch(`/${encodeURIComponent(bucket)}/${encodeURIComponent(key)}`, {
                method: 'PUT',
                body: file
            });

            if (response.ok) {
                app.showToast('success', 'Upload Successful', `Object "${key}" has been uploaded successfully.`);
                app.loadObjectsList(bucket);
            } else {
                const status = response.status;
                let errorMessage = `Failed to upload object`;

                if (status === 404) {
                    errorMessage = `Bucket "${bucket}" not found.`;
                } else if (status === 413) {
                    errorMessage = `File too large. Maximum size is 5GB.`;
                } else {
                    errorMessage = `Failed to upload object: ${status}`;
                }

                app.showToast('error', 'Upload Failed', errorMessage);
            }
        } catch (error) {
            app.showToast('error', 'Error', error.message);
        }
    };
    input.click();
}

async function deleteObject(bucket, key) {
    const confirmed = await app.showModal({
        type: 'confirm',
        title: 'Delete Object',
        message: `Are you sure you want to delete "${key}" from bucket "${bucket}"?`,
        warning: 'This action cannot be undone.',
        confirmText: 'Delete',
        cancelText: 'Cancel',
        danger: true
    });

    if (!confirmed) return;

    try {
        const response = await fetch(`/${encodeURIComponent(bucket)}/${encodeURIComponent(key)}`, {
            method: 'DELETE'
        });

        if (response.ok) {
            app.showToast('success', 'Object Deleted', `Object "${key}" has been deleted successfully.`);
            app.loadObjectsList(bucket);
        } else {
            const status = response.status;
            let errorMessage = `Failed to delete object`;

            if (status === 404) {
                errorMessage = `Object "${key}" not found in bucket "${bucket}".`;
            } else {
                errorMessage = `Failed to delete object: ${status}`;
            }

            app.showToast('error', 'Deletion Failed', errorMessage);
        }
    } catch (error) {
        app.showToast('error', 'Error', error.message);
    }
}

async function downloadObject(bucket, key) {
    try {
        const response = await fetch(`/${encodeURIComponent(bucket)}/${encodeURIComponent(key)}`);

        if (response.ok) {
            const blob = await response.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = key.split('/').pop();
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(url);
            document.body.removeChild(a);
            app.showToast('info', 'Download Started', `Downloading "${key}"...`);
        } else {
            const status = response.status;
            let errorMessage = `Failed to download object`;

            if (status === 404) {
                errorMessage = `Object "${key}" not found in bucket "${bucket}".`;
            } else {
                errorMessage = `Failed to download object: ${status}`;
            }

            app.showToast('error', 'Download Failed', errorMessage);
        }
    } catch (error) {
        app.showToast('error', 'Error', error.message);
    }
}

// Initialize app
const app = new ObjStorApp();
document.addEventListener('DOMContentLoaded', () => app.init());
