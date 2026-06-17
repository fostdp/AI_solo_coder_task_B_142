class DeviceCompare {
    constructor() {
        this.deviceTypes = [];
        this.compareChart = null;
    }

    async init(deviceTypes) {
        this.deviceTypes = deviceTypes || [];
        this.renderDeviceCheckboxes();
        this.bindCompareEvents();
    }

    renderDeviceCheckboxes() {
        const container = document.getElementById('deviceCheckboxes');
        if (!container) return;

        const defaults = ['sinan', 'zhinanyu', 'han_luopan', 'mems_compass'];
        container.innerHTML = this.deviceTypes.map(d => `
            <label class="checkbox-item">
                <input type="checkbox" value="${d.device_type}"
                    ${defaults.includes(d.device_type) ? 'checked' : ''}>
                <span class="cb-name">${d.display_name}</span>
                <span class="cb-era">${d.era}</span>
            </label>
        `).join('');
    }

    bindCompareEvents() {
        const btn = document.getElementById('runCompareBtn');
        if (btn) {
            btn.addEventListener('click', () => this.runDeviceComparison());
        }
    }

    async runDeviceComparison() {
        const targetYear = parseFloat(document.getElementById('cmpTargetYear').value);
        const lat = parseFloat(document.getElementById('cmpLat').value);
        const lon = parseFloat(document.getElementById('cmpLon').value);
        const temperature = parseFloat(document.getElementById('cmpTemp').value);
        const expectedAzimuth = parseFloat(document.getElementById('cmpAzimuth').value);

        const devices = Array.from(document.querySelectorAll('#deviceCheckboxes input:checked'))
            .map(cb => cb.value);

        if (devices.length === 0) {
            showToast('请至少选择一个装置', 'error');
            return;
        }

        showToast('正在运行对比仿真...');
        try {
            const resp = await dataService.compareDevices({
                target_year: targetYear,
                location_lat: lat,
                location_lon: lon,
                devices,
                temperature,
                expected_azimuth: expectedAzimuth,
            });
            this.renderDeviceComparisonResults(resp.data);
            showToast('多装置对比完成', 'success');
        } catch (e) {
            showToast('对比仿真失败: ' + e.message, 'error');
        }
    }

    renderDeviceComparisonResults(data) {
        const container = document.getElementById('compareResults');
        if (!container) return;

        const rankingHtml = data.ranking.map((dt, i) => {
            const dev = data.devices.find(d => d.device_type === dt);
            const emoji = i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : `${i + 1}.`;
            return `<div class="rank-item ${i === 0 ? 'rank-best' : ''}">
                <span class="rank-index">${emoji}</span>
                <span class="rank-name">${dev ? dev.display_name : dt}</span>
                <span class="rank-accuracy">平均偏差 ${dev ? dev.mean_deviation_deg.toFixed(2) : '--'}°</span>
            </div>`;
        }).join('');

        const devicesHtml = data.devices.map(d => `
            <div class="device-card">
                <div class="device-card-header">
                    <h4>${d.display_name}</h4>
                    <span class="device-era">${d.era}</span>
                </div>
                <div class="accuracy-bar">
                    <div class="accuracy-bar-fill" style="width:${Math.min(d.mean_deviation_deg * 5, 100)}%"></div>
                </div>
                <div class="device-stats">
                    <div><span class="stat-label">平均偏差</span><span class="stat-value">${d.mean_deviation_deg.toFixed(2)}°</span></div>
                    <div><span class="stat-label">标准差</span><span class="stat-value">${d.std_deviation_deg.toFixed(2)}°</span></div>
                    <div><span class="stat-label">P95偏差</span><span class="stat-value">${d.p95_deviation_deg.toFixed(2)}°</span></div>
                    <div><span class="stat-label">范围</span><span class="stat-value">${d.min_deviation_deg.toFixed(1)}° ~ ${d.max_deviation_deg.toFixed(1)}°</span></div>
                </div>
                <p class="device-notes">${d.notes}</p>
            </div>
        `).join('');

        container.innerHTML = `
            <div class="compare-summary">
                <h4>地磁场条件</h4>
                <p>年份: ${data.target_year} | 位置: ${data.location_lat.toFixed(3)}°N, ${data.location_lon.toFixed(3)}°E
                   | 强度: ${data.geomagnetic_intensity_nT.toFixed(0)} nT
                   | 磁偏角: ${data.geomagnetic_declination_deg.toFixed(2)}°
                   | 磁倾角: ${data.geomagnetic_inclination_deg.toFixed(2)}°</p>
            </div>
            <div class="ranking-panel">
                <h4>精度排名</h4>
                ${rankingHtml}
            </div>
            <p class="compare-narrative">${data.summary}</p>
            <div class="device-cards">${devicesHtml}</div>
            <canvas id="compareChart" width="800" height="350"></canvas>
        `;

        this.renderCompareChart(data);
    }

    renderCompareChart(data) {
        const canvas = document.getElementById('compareChart');
        if (!canvas || !window.Chart) return;

        const labels = data.devices.map(d => d.display_name);
        const means = data.devices.map(d => d.mean_deviation_deg);
        const p95 = data.devices.map(d => d.p95_deviation_deg);
        const stds = data.devices.map(d => d.std_deviation_deg);

        if (this.compareChart) this.compareChart.destroy();
        this.compareChart = new Chart(canvas.getContext('2d'), {
            type: 'bar',
            data: {
                labels,
                datasets: [
                    { label: '平均偏差 (°)', data: means, backgroundColor: 'rgba(255, 159, 64, 0.7)', borderColor: '#ff9f40', borderWidth: 1 },
                    { label: 'P95偏差 (°)', data: p95, backgroundColor: 'rgba(255, 99, 132, 0.5)', borderColor: '#ff6384', borderWidth: 1 },
                    { label: '标准差 (°)', data: stds, type: 'line', borderColor: '#36a2eb', backgroundColor: '#36a2eb', fill: false, tension: 0.3 },
                ]
            },
            options: {
                responsive: true,
                plugins: { title: { display: true, text: '古代指向装置精度对比' } },
                scales: { y: { beginAtZero: true, title: { display: true, text: '角度 (°)' } } }
            }
        });
    }
}

const deviceCompare = new DeviceCompare();
