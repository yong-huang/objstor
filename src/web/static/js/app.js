class ObjStorApp {
    constructor() {
        this.ws = null;
        this.currentPage = 'dashboard';
        this.charts = {};
        this.startTime = Date.now();
        this.currentTheme = localStorage.getItem('theme') || 'light';
        // Data history for trend charts
        this.history = {
            objectCounts: [],
            requestTimestamps: [],
            storageUsed: [],
            poolObjects: {},
        };
        this.systemMetricsInterval = null;
    }

    async init() {
        this.setupTheme();
        this.setupNavigation();
        this.setupConfigActions();
        this.connectWebSocket();
        await this.loadPage('dashboard');
        this.updateUptime();
        setInterval(() => this.updateUptime(), 1000);
    }

    setupTheme() {
        // Apply saved theme
        document.documentElement.setAttribute('data-theme', this.currentTheme);
        this.updateThemeIcon();

        // Setup theme toggle button
        const toggleBtn = document.getElementById('theme-toggle');
        if (toggleBtn) {
            toggleBtn.addEventListener('click', () => this.toggleTheme());
        }
    }

    toggleTheme() {
        this.currentTheme = this.currentTheme === 'light' ? 'dark' : 'light';
        document.documentElement.setAttribute('data-theme', this.currentTheme);
        localStorage.setItem('theme', this.currentTheme);
        this.updateThemeIcon();

        // Show toast notification
        this.showToast(
            'info',
            'Theme Changed',
            `Switched to ${this.currentTheme} mode`,
            2000
        );
    }

    updateThemeIcon() {
        const sunIcon = document.getElementById('theme-icon-sun');
        const moonIcon = document.getElementById('theme-icon-moon');

        if (this.currentTheme === 'dark') {
            sunIcon.style.display = 'none';
            moonIcon.style.display = 'block';
        } else {
            sunIcon.style.display = 'block';
            moonIcon.style.display = 'none';
        }
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

    setupConfigActions() {
        const saveBtn = document.getElementById('save-config-btn');
        if (saveBtn) {
            saveBtn.addEventListener('click', () => this.saveConfiguration());
        }

        const addKeyBtn = document.getElementById('add-access-key-btn');
        if (addKeyBtn) {
            addKeyBtn.addEventListener('click', () => this.showAccessKeyModal());
        }
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

        // Clean up system metrics polling when leaving monitoring page
        if (page !== 'monitoring' && this.systemMetricsInterval) {
            clearInterval(this.systemMetricsInterval);
            this.systemMetricsInterval = null;
        }

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
            case 'settings':
                await this.loadSettings();
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
                        this.updateDashboardCharts(data.data);
                    }
                    if (this.currentPage === 'monitoring') {
                        this.updateMonitoringCharts(data.data);
                    }
                }
                break;
            case 'log':
                if (this.currentPage === 'logs') {
                    this.appendLog(data.data);
                }
                break;
            case 'event':
                this.handleEvent(data);
                break;
            default:
                console.log('Unknown message type:', data.type);
        }
    }

    handleEvent(data) {
        if (!data.event || !data.data) return;
        const eventName = data.event;
        const eventData = data.data;
        let title = '';
        let message = '';
        let toastType = 'info';

        switch (eventName) {
            case 'ObjectCreated':
                title = 'Object Created';
                message = `${eventData.bucket}/${eventData.key} (${this.formatBytes(eventData.size || 0)})`;
                toastType = 'success';
                break;
            case 'ObjectDeleted':
                title = 'Object Deleted';
                message = `${eventData.bucket}/${eventData.key}`;
                toastType = 'warning';
                break;
            case 'BucketCreated':
                title = 'Bucket Created';
                message = eventData.bucket;
                toastType = 'success';
                break;
            case 'BucketDeleted':
                title = 'Bucket Deleted';
                message = eventData.bucket;
                toastType = 'warning';
                break;
            case 'IntegrityCheck':
                title = 'Integrity Check';
                if (eventData.mismatches > 0) {
                    message = `${eventData.mismatches} mismatches found`;
                    toastType = 'error';
                } else {
                    message = `All ${eventData.checked} objects verified OK`;
                    toastType = 'success';
                }
                break;
            default:
                title = eventName;
                message = JSON.stringify(eventData);
        }

        this.showToast(toastType, title, message, 4000);
    }

    async loadDashboard() {
        try {
            const response = await fetch('/api/v1/metrics');
            const data = await response.json();
            this.updateMetrics(data);
            this.updateDashboardSummary(data);
            this.initPoolDonut(data);
            this.initStorageClassChart();
        } catch (error) {
            console.error('Failed to load dashboard:', error);
        }
    }

    updateMetrics(data) {
        if (!data) return;

        // Record history
        const now = Date.now();
        this.history.storageUsed.push((data.storage?.used || 0) / (1024 * 1024 * 1024));
        this.history.objectCounts.push(data.total_objects || 0);
        this.history.requestTimestamps.push(now);
        if (data.pools) {
            data.pools.forEach(p => {
                if (!this.history.poolObjects[p.id]) this.history.poolObjects[p.id] = [];
                this.history.poolObjects[p.id].push(p.objects || 0);
            });
        }
        // Keep last 60 data points
        const maxLen = 60;
        if (this.history.storageUsed.length > maxLen) this.history.storageUsed.shift();
        if (this.history.objectCounts.length > maxLen) this.history.objectCounts.shift();
        if (this.history.requestTimestamps.length > maxLen) this.history.requestTimestamps.shift();
        Object.values(this.history.poolObjects).forEach(arr => {
            while (arr.length > maxLen) arr.shift();
        });
    }

    updateDashboardSummary(data) {
        if (!data) return;
        const storage = data.storage || {};
        const used = this.formatBytes(storage.used || 0);
        const capacity = this.formatBytes(storage.capacity || 0);

        const storageEl = document.getElementById('total-storage');
        if (storageEl) storageEl.textContent = `${used} / ${capacity}`;

        const bucketCount = data.buckets?.length || 0;
        const bucketEl = document.getElementById('bucket-count');
        if (bucketEl) bucketEl.textContent = bucketCount;

        const objectCount = data.total_objects || 0;
        const objEl = document.getElementById('object-count');
        if (objEl) objEl.textContent = objectCount.toLocaleString();

        this.updatePoolsList(data.pools || []);
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

    updateDashboardCharts(metricsData) {
        this.updateDashboardSummary(metricsData);
        this.updatePoolDonut(metricsData);
        this.updateStorageClassChart();
    }

    initPoolDonut(metricsData) {
        const ctx = document.getElementById('pool-donut-chart');
        if (!ctx) return;
        if (this.charts.poolDonut) this.charts.poolDonut.destroy();

        const pools = metricsData?.pools || [];
        if (pools.length === 0) return;

        const colors = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899'];

        this.charts.poolDonut = new Chart(ctx, {
            type: 'doughnut',
            data: {
                labels: pools.map(p => p.id),
                datasets: [{
                    data: pools.map(p => ((p.used || 0) / (1024 * 1024)).toFixed(2)),
                    backgroundColor: colors.slice(0, pools.length),
                    borderWidth: 0,
                    hoverOffset: 6,
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                cutout: '65%',
                plugins: {
                    legend: { position: 'right', labels: { boxWidth: 12, padding: 12, font: { size: 11 } } },
                    tooltip: {
                        callbacks: {
                            label: (ctx) => `${ctx.label}: ${ctx.parsed} MB`
                        }
                    }
                }
            }
        });
    }

    initStorageClassChart() {
        const ctx = document.getElementById('storage-class-chart');
        if (!ctx) return;
        if (this.charts.storageClass) this.charts.storageClass.destroy();

        this.charts.storageClass = new Chart(ctx, {
            type: 'doughnut',
            data: {
                labels: [],
                datasets: [{
                    data: [],
                    backgroundColor: ['#3b82f6', '#10b981', '#f59e0b', '#ef4444'],
                    borderWidth: 0,
                    hoverOffset: 6,
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                cutout: '65%',
                plugins: {
                    legend: { position: 'right', labels: { boxWidth: 12, padding: 12, font: { size: 11 } } },
                    tooltip: {
                        callbacks: {
                            label: (ctx) => `${ctx.label}: ${ctx.parsed}`
                        }
                    }
                }
            }
        });
    }

    updateMonitoringCharts(metricsData) {
        // Storage usage trend
        if (this.charts.storage) {
            const chart = this.charts.storage;
            const idx = this.history.storageUsed.length - 1;
            chart.data.labels.push(idx);
            chart.data.datasets[0].data.push((this.history.storageUsed[idx] || 0).toFixed(2));
            if (chart.data.labels.length > 30) { chart.data.labels.shift(); chart.data.datasets[0].data.shift(); }
            chart.update('none');
        }

        // Requests per minute
        this.updateRequestsChart();

        // Object count trend
        if (this.charts.objectTrend) {
            const trend = this.charts.objectTrend;
            const now = new Date().toLocaleTimeString();
            trend.data.labels.push(now);
            trend.data.datasets[0].data.push(metricsData.total_objects || 0);
            if (trend.data.labels.length > 30) { trend.data.labels.shift(); trend.data.datasets[0].data.shift(); }
            trend.update('none');
        }

        // Pool objects bar
        if (this.charts.poolObjects && metricsData.pools) {
            const poolChart = this.charts.poolObjects;
            poolChart.data.labels = metricsData.pools.map(p => p.id || 'unknown');
            poolChart.data.datasets[0].data = metricsData.pools.map(p => p.objects || 0);
            poolChart.data.datasets[1].data = metricsData.pools.map(p =>
                ((p.used || 0) / (1024 * 1024)).toFixed(2)
            );
            poolChart.update('none');
        }
    }

    updateRequestsChart() {
        if (!this.charts.requests) return;
        const chart = this.charts.requests;
        const now = Date.now();
        const oneMinAgo = now - 60000;
        const recent = this.history.requestTimestamps.filter(t => t > oneMinAgo);
        // Bucket into 10-second windows
        const buckets = {};
        recent.forEach(t => {
            const slot = Math.floor(t / 10000) * 10;
            buckets[slot] = (buckets[slot] || 0) + 1;
        });
        const labels = Object.keys(buckets).map(k => {
            const mins = Math.floor((parseInt(k) * 10) / 60);
            const secs = (parseInt(k) * 10) % 60;
            return `${mins}m${secs < 10 ? '0' : ''}${secs}s`;
        });
        chart.data.labels = labels;
        chart.data.datasets[0].data = Object.values(buckets);
        chart.update('none');
    }

    updatePoolDonut(metricsData) {
        if (!this.charts.poolDonut) return;
        const pools = metricsData?.pools || [];
        if (pools.length === 0) return;
        this.charts.poolDonut.data.datasets[0].data = pools.map(p => ((p.used || 0) / (1024 * 1024)).toFixed(2));
        this.charts.poolDonut.update('none');
    }

    updateStorageClassChart() {
        if (!this.charts.storageClass) return;
        // Count objects per storage class from pool data (aggregate from all pools)
        const classMap = {};
        (this.charts.poolDonut?.data?.labels || []).forEach(id => id);
        // Use buckets data to get storage class counts from DB — for now estimate from pool tiers
        const pools = this.charts.poolDonut?.data?.labels || [];
        const tierClassMap = { hot: 'STANDARD', warm: 'STANDARD_IA', cold: 'GLACIER' };
        pools.forEach(id => { classMap[tierClassMap[id] || id] = (classMap[tierClassMap[id] || id] || 0) + 1; });
        const labels = Object.keys(classMap);
        const data = Object.values(classMap);
        if (labels.length === 0) return;
        this.charts.storageClass.data.labels = labels;
        this.charts.storageClass.data.datasets[0].data = data;
        this.charts.storageClass.update('none');
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

    formatBytes(bytes) {
        const units = ['B', 'KB', 'MB', 'GB', 'TB'];
        let i = 0;
        let size = bytes;
        while (size >= 1024 && i < units.length - 1) {
            size /= 1024;
            i++;
        }
        return size.toFixed(i === 0 ? 0 : 1) + ' ' + units[i];
    }

    formatUptime(secs) {
        const days = Math.floor(secs / 86400);
        const hours = Math.floor((secs % 86400) / 3600);
        const mins = Math.floor((secs % 3600) / 60);
        if (days > 0) return `${days}d ${hours}h`;
        if (hours > 0) return `${hours}h ${mins}m`;
        return `${mins}m`;
    }

    setBarThreshold(barEl, percent) {
        barEl.classList.remove('warn', 'critical');
        if (percent >= 85) {
            barEl.classList.add('critical');
        } else if (percent >= 60) {
            barEl.classList.add('warn');
        }
    }

    async loadSystemMetrics() {
        try {
            const res = await fetch('/api/v1/system');
            const data = await res.json();
            const cpu = data.cpu || {};
            const mem = data.memory || {};
            const disk = data.disk || {};
            const sys = data.system || {};

            // CPU
            const cpuEl = document.getElementById('sys-cpu-value');
            if (cpuEl) cpuEl.textContent = (cpu.usage_percent || 0).toFixed(1) + '%';
            const cpuBar = document.getElementById('sys-cpu-bar');
            if (cpuBar) {
                cpuBar.style.width = (cpu.usage_percent || 0) + '%';
                this.setBarThreshold(cpuBar, cpu.usage_percent || 0);
            }
            const cpuDetail = document.getElementById('sys-cpu-detail');
            if (cpuDetail) cpuDetail.textContent = `${cpu.brand || '--'} / ${cpu.cores || '--'} cores`;

            // Memory
            const memEl = document.getElementById('sys-mem-value');
            if (memEl) memEl.textContent = (mem.percent || 0).toFixed(1) + '%';
            const memBar = document.getElementById('sys-mem-bar');
            if (memBar) {
                memBar.style.width = (mem.percent || 0) + '%';
                this.setBarThreshold(memBar, mem.percent || 0);
            }
            const memDetail = document.getElementById('sys-mem-detail');
            if (memDetail) memDetail.textContent = `${this.formatBytes(mem.used || 0)} / ${this.formatBytes(mem.total || 0)}`;

            // Disk
            const diskEl = document.getElementById('sys-disk-value');
            if (diskEl) diskEl.textContent = (disk.percent || 0).toFixed(1) + '%';
            const diskBar = document.getElementById('sys-disk-bar');
            if (diskBar) {
                diskBar.style.width = (disk.percent || 0) + '%';
                this.setBarThreshold(diskBar, disk.percent || 0);
            }
            const diskDetail = document.getElementById('sys-disk-detail');
            if (diskDetail) diskDetail.textContent = `${this.formatBytes(disk.used || 0)} / ${this.formatBytes(disk.total || 0)}`;

            // System info
            const osEl = document.getElementById('sys-os');
            if (osEl) osEl.textContent = sys.os || '--';
            const kernelEl = document.getElementById('sys-kernel');
            if (kernelEl) kernelEl.textContent = sys.kernel || '--';
            const uptimeEl = document.getElementById('sys-uptime');
            if (uptimeEl) uptimeEl.textContent = this.formatUptime(sys.uptime_secs || 0);
            const hostnameEl = document.getElementById('sys-hostname');
            if (hostnameEl) hostnameEl.textContent = sys.hostname || '--';
        } catch (error) {
            console.error('Failed to load system metrics:', error);
        }
    }

    async loadMonitoring() {
        // Load system resource metrics
        await this.loadSystemMetrics();
        // Poll every 5 seconds
        if (this.systemMetricsInterval) clearInterval(this.systemMetricsInterval);
        this.systemMetricsInterval = setInterval(() => this.loadSystemMetrics(), 5000);

        // Storage Usage Trend
        const storageCtx = document.getElementById('storage-chart');
        if (storageCtx) {
            if (this.charts.storage) this.charts.storage.destroy();
            this.charts.storage = new Chart(storageCtx, {
                type: 'line',
                data: {
                    labels: this.history.storageUsed.map((_, i) => i),
                    datasets: [{
                        label: 'Storage Used (GB)',
                        data: [...this.history.storageUsed.map(v => v.toFixed(2))],
                        borderColor: '#3b82f6',
                        backgroundColor: 'rgba(59, 130, 246, 0.1)',
                        fill: true,
                        tension: 0.4,
                        pointRadius: 0,
                        borderWidth: 2,
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {
                        x: { display: false },
                        y: { beginAtZero: true, title: { display: true, text: 'GB' } }
                    },
                    plugins: { legend: { display: false } }
                }
            });
        }

        // Requests per minute
        const reqCtx = document.getElementById('requests-chart');
        if (reqCtx) {
            if (this.charts.requests) this.charts.requests.destroy();
            this.charts.requests = new Chart(reqCtx, {
                type: 'line',
                data: {
                    labels: [],
                    datasets: [{
                        label: 'Requests',
                        data: [],
                        borderColor: 'rgba(99, 102, 241, 1)',
                        backgroundColor: 'rgba(99, 102, 241, 0.15)',
                        fill: true,
                        tension: 0.4,
                        pointRadius: 0,
                        borderWidth: 2,
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {
                        x: { display: false },
                        y: { beginAtZero: true, title: { display: true, text: 'Requests' } }
                    },
                    plugins: { legend: { display: false } }
                }
            });
        }

        // Object Count Trend
        const trendCtx = document.getElementById('object-trend-chart');
        if (trendCtx) {
            if (this.charts.objectTrend) this.charts.objectTrend.destroy();
            this.charts.objectTrend = new Chart(trendCtx, {
                type: 'line',
                data: {
                    labels: [],
                    datasets: [{
                        label: 'Total Objects',
                        data: [],
                        borderColor: '#10b981',
                        backgroundColor: 'rgba(16, 185, 129, 0.1)',
                        fill: true,
                        tension: 0.4,
                        pointRadius: 2
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: { y: { beginAtZero: true } },
                    plugins: { legend: { display: true } }
                }
            });
        }

        // Pool Objects bar chart
        const poolCtx = document.getElementById('pool-objects-chart');
        if (poolCtx) {
            if (this.charts.poolObjects) this.charts.poolObjects.destroy();
            this.charts.poolObjects = new Chart(poolCtx, {
                type: 'bar',
                data: {
                    labels: [],
                    datasets: [
                        {
                            label: 'Objects',
                            data: [],
                            backgroundColor: 'rgba(59, 130, 246, 0.7)',
                            borderColor: '#3b82f6',
                            borderWidth: 1
                        },
                        {
                            label: 'Used (MB)',
                            data: [],
                            backgroundColor: 'rgba(16, 185, 129, 0.7)',
                            borderColor: '#10b981',
                            borderWidth: 1
                        }
                    ]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: { y: { beginAtZero: true } },
                    plugins: { legend: { display: true } }
                }
            });
        }

        // Load initial data
        try {
            const response = await fetch('/api/v1/metrics');
            const data = await response.json();
            this.updateMetrics(data);
            this.updateMonitoringCharts(data);
        } catch (error) {
            console.error('Failed to load monitoring data:', error);
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

    calculateUptime() {
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

        return uptimeText;
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

    async loadSettings() {
        // Update system info
        const uptimeElement = document.getElementById('settings-uptime');
        const startedElement = document.getElementById('settings-started');

        if (uptimeElement) {
            const uptime = this.calculateUptime();
            uptimeElement.textContent = uptime;
        }

        if (startedElement) {
            const started = new Date(this.startTime).toLocaleString();
            startedElement.textContent = started;
        }

        // Load configuration
        await this.loadConfiguration();

        // Load access keys
        await this.loadAccessKeys();
    }

    async loadConfiguration() {
        try {
            const response = await fetch('/api/v1/config');
            if (!response.ok) throw new Error('Failed to load configuration');

            const config = await response.json();

            // Update server config fields
            if (config.server) {
                document.getElementById('config-server-host').value = config.server.host || '0.0.0.0';
                document.getElementById('config-server-port').value = config.server.port || 8080;
                document.getElementById('config-s3-port').value = config.server.s3_port || 8080;
                document.getElementById('config-log-level').value = config.server.log_level || 'info';
                document.getElementById('config-log-dir').value = config.server.log_dir || './logs';
                document.getElementById('config-max-request-size').value = config.server.max_request_size || 5368709120;
            }

            // Update storage config fields
            if (config.storage) {
                document.getElementById('config-scheduler-strategy').value = config.storage.scheduler?.strategy || 'least_loaded';
                document.getElementById('config-rebalance-threshold').value = config.storage.scheduler?.rebalance_threshold || 0.2;
            }
        } catch (error) {
            console.error('Failed to load configuration:', error);
            this.showToast('error', 'Error', 'Failed to load configuration');
        }
    }

    // ===== Access Key Management =====

    async loadAccessKeys() {
        const container = document.getElementById('access-keys-list');
        if (!container) return;
        try {
            const res = await fetch('/api/v1/access-keys');
            const data = await res.json();
            const keys = data.keys || [];
            if (keys.length === 0) {
                container.innerHTML = '<div class="ak-empty">No access keys configured</div>';
                return;
            }
            container.innerHTML = keys.map(k => `
                <div class="ak-item" data-key-id="${this.escHtml(k.access_key_id)}">
                    <div class="ak-info">
                        <div class="ak-id">${this.escHtml(k.access_key_id)}</div>
                        <div class="ak-meta">Created ${this.escHtml(k.created_at || '')} &middot; ${this.escHtml(k.status || '')}</div>
                    </div>
                    <div class="ak-actions">
                        <button class="ak-btn" onclick="app.editAccessKey('${this.escHtml(k.access_key_id)}')">Edit</button>
                        <button class="ak-btn ak-btn-danger" onclick="app.deleteAccessKey('${this.escHtml(k.access_key_id)}')">Delete</button>
                    </div>
                </div>
            `).join('');
        } catch (e) {
            container.innerHTML = '<div class="ak-empty">Failed to load access keys</div>';
        }
    }

    showAccessKeyModal(existingId, existingSecret) {
        const title = existingId ? 'Edit Access Key' : 'Add Access Key';
        const idValue = existingId || '';
        const secretValue = existingSecret || '';
        const body = document.getElementById('modal-body');
        body.innerHTML = `
            <div class="modal-form-group">
                <label>Access Key ID</label>
                <input type="text" id="modal-ak-id" value="${this.escHtml(idValue)}" placeholder="e.g. my-access-key" ${existingId ? 'readonly style="opacity:0.6;cursor:not-allowed;"' : ''}>
                ${existingId ? '' : '<div class="modal-form-hint">Cannot be changed after creation</div>'}
            </div>
            <div class="modal-form-group">
                <label>Secret Key</label>
                <input type="text" id="modal-ak-secret" value="${this.escHtml(secretValue)}" placeholder="Enter new secret key">
            </div>
            <div class="modal-form-actions">
                <button class="btn-modal-cancel" onclick="app.closeModal()">Cancel</button>
                <button class="btn-modal-save" onclick="app.saveAccessKey('${this.escHtml(idValue)}', ${!!existingId})">Save</button>
            </div>
        `;
        document.getElementById('modal-title').textContent = title;
        document.getElementById('modal-overlay').style.display = 'flex';
    }

    editAccessKey(keyId) {
        this.showAccessKeyModal(keyId, '');
    }

    async saveAccessKey(originalId, isEdit) {
        const id = document.getElementById('modal-ak-id').value.trim();
        const secret = document.getElementById('modal-ak-secret').value.trim();
        if (!id || !secret) {
            this.showToast('error', 'Error', 'Access Key ID and Secret Key are required');
            return;
        }
        try {
            let res;
            if (isEdit) {
                res = await fetch(`/api/v1/access-keys/${encodeURIComponent(id)}`, {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ secret_key: secret }),
                });
            } else {
                res = await fetch('/api/v1/access-keys', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ access_key_id: id, secret_key: secret }),
                });
            }
            if (!res.ok) {
                const err = await res.json().catch(() => ({}));
                throw new Error(err.error || 'Failed to save');
            }
            this.closeModal();
            this.showToast('success', 'Access Key', isEdit ? 'Secret key updated' : 'Access key created');
            await this.loadAccessKeys();
        } catch (e) {
            this.showToast('error', 'Error', e.message);
        }
    }

    async deleteAccessKey(keyId) {
        if (!confirm(`Delete access key "${keyId}"?`)) return;
        try {
            const res = await fetch(`/api/v1/access-keys/${encodeURIComponent(keyId)}`, { method: 'DELETE' });
            if (!res.ok) throw new Error('Failed to delete');
            this.showToast('success', 'Access Key', `Deleted "${keyId}"`);
            await this.loadAccessKeys();
        } catch (e) {
            this.showToast('error', 'Error', e.message);
        }
    }

    escHtml(s) {
        const d = document.createElement('div');
        d.textContent = s;
        return d.innerHTML;
    }

    async saveConfiguration() {
        const saveBtn = document.getElementById('save-config-btn');
        const originalText = saveBtn.innerHTML;

        try {
            // Disable button and show loading
            saveBtn.disabled = true;
            saveBtn.innerHTML = `
                <svg viewBox="0 0 24 24" style="width: 1.25rem; height: 1.25rem; margin-right: 0.5rem; animation: spin 1s linear infinite;">
                    <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" opacity="0.25"/>
                    <path d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" fill="currentColor"/>
                </svg>
                Saving...
            `;

            // Gather configuration data
            const config = {
                server: {
                    host: document.getElementById('config-server-host').value,
                    port: parseInt(document.getElementById('config-server-port').value),
                    s3_port: parseInt(document.getElementById('config-s3-port').value),
                    log_level: document.getElementById('config-log-level').value,
                    log_dir: document.getElementById('config-log-dir').value,
                    max_request_size: parseInt(document.getElementById('config-max-request-size').value),
                },
                storage: {
                    scheduler: {
                        strategy: document.getElementById('config-scheduler-strategy').value,
                        rebalance_threshold: parseFloat(document.getElementById('config-rebalance-threshold').value),
                    }
                }
            };

            // Send update request
            const response = await fetch('/api/v1/config', {
                method: 'PUT',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(config)
            });

            const result = await response.json();

            if (result.success) {
                this.showToast('success', 'Success', result.message || 'Configuration saved successfully!');
            } else {
                throw new Error(result.error || 'Failed to save configuration');
            }
        } catch (error) {
            console.error('Failed to save configuration:', error);
            this.showToast('error', 'Error', error.message || 'Failed to save configuration');
        } finally {
            // Restore button
            saveBtn.disabled = false;
            saveBtn.innerHTML = originalText;
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
