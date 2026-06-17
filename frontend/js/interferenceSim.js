class InterferenceSimulator {
    constructor() {
        this.interferenceTypes = [];
        this.interferenceChart = null;
    }

    init(interferenceTypes) {
        this.interferenceTypes = interferenceTypes || [];
        this.renderInterferenceTypeOptions();
        this.bindInterferenceEvents();
    }

    renderInterferenceTypeOptions() {
        const container = document.getElementById('interferenceList');
        if (!container) return;
        container.innerHTML = '';
        this.addInterferenceRow();
    }

    addInterferenceRow() {
        const container = document.getElementById('interferenceList');
        if (!container) return;

        const options = this.interferenceTypes.map(t =>
            `<option value="${t.interference_type}">${t.display_name}</option>`
        ).join('');

        const row = document.createElement('div');
        row.className = 'interference-row';
        row.innerHTML = `
            <select class="inf-type">${options}</select>
            <input type="number" class="inf-distance" value="1.0" step="0.1" min="0.01" placeholder="距离(m)">
            <input type="number" class="inf-factor" value="1.0" step="0.1" min="0" placeholder="强度系数">
            <input type="number" class="inf-azimuth" value="0" step="1" min="0" max="360" placeholder="方位°">
            <button class="btn btn-danger btn-sm remove-inf">×</button>
        `;
        row.querySelector('.remove-inf').addEventListener('click', () => row.remove());
        container.appendChild(row);
    }

    bindInterferenceEvents() {
        const addBtn = document.getElementById('addInterferenceBtn');
        if (addBtn) {
            addBtn.addEventListener('click', () => this.addInterferenceRow());
        }
        const runBtn = document.getElementById('runInterferenceBtn');
        if (runBtn) {
            runBtn.addEventListener('click', () => this.runInterferenceSimulation());
        }
    }

    async runInterferenceSimulation() {
        const deviceType = document.getElementById('infDeviceType').value;
        const targetYear = parseFloat(document.getElementById('infYear').value);
        const lat = parseFloat(document.getElementById('infLat').value);
        const lon = parseFloat(document.getElementById('infLon').value);

        const rows = document.querySelectorAll('#interferenceList .interference-row');
        const sources = [];
        rows.forEach(row => {
            const type = row.querySelector('.inf-type').value;
            const dist = parseFloat(row.querySelector('.inf-distance').value);
            const factor = parseFloat(row.querySelector('.inf-factor').value);
            const azimuth = parseFloat(row.querySelector('.inf-azimuth').value);
            if (type && !isNaN(dist) && !isNaN(factor) && !isNaN(azimuth)) {
                sources.push({
                    interference_type: type,
                    distance_m: dist,
                    intensity_factor: factor,
                    azimuth_deg: azimuth,
                });
            }
        });

        if (sources.length === 0) {
            showToast('请至少配置一个干扰源', 'error');
            return;
        }

        showToast('正在运行干扰仿真...');
        try {
            const resp = await dataService.simulateInterference({
                device_type: deviceType,
                target_year: targetYear,
                location_lat: lat,
                location_lon: lon,
                temperature: 25,
                expected_azimuth: 180,
                interference_sources: sources,
            });
            this.renderInterferenceResults(resp.data);
            showToast('干扰仿真完成', 'success');
        } catch (e) {
            showToast('干扰仿真失败: ' + e.message, 'error');
        }
    }

    renderInterferenceResults(data) {
        const container = document.getElementById('interferenceResults');
        if (!container) return;

        const warnClass = data.warning_level === '严重干扰' ? 'warn-severe'
            : data.warning_level === '中度干扰' ? 'warn-moderate'
            : data.warning_level === '轻微干扰' ? 'warn-mild' : 'warn-safe';

        const effectsHtml = data.effects.map(e => `
            <div class="effect-item">
                <div class="effect-name">${e.display_name}</div>
                <div class="effect-details">
                    <span>距离 ${e.distance_m} m</span>
                    <span>干扰场 ${e.induced_field_nT.toFixed(1)} nT</span>
                    <span>方位 ${e.induced_field_azimuth_deg.toFixed(0)}°</span>
                    <span class="effect-dev">偏差贡献 ${e.deviation_contribution_deg.toFixed(2)}°</span>
                </div>
            </div>
        `).join('');

        container.innerHTML = `
            <div class="interference-summary ${warnClass}">
                <h4>干扰等级: <span class="warn-level">${data.warning_level}</span></h4>
                <p>${data.recommendation}</p>
            </div>
            <div class="interference-compare">
                <div class="baseline">
                    <div class="ic-label">无干扰基准</div>
                    <div class="ic-azimuth">方位 ${data.baseline_azimuth.toFixed(1)}°</div>
                    <div class="ic-dev">偏差 ${data.baseline_accuracy_deg.toFixed(2)}°</div>
                </div>
                <div class="delta-arrow">
                    <span>Δ = ${data.total_deviation_delta_deg.toFixed(2)}°</span>
                </div>
                <div class="interfered">
                    <div class="ic-label">受干扰后</div>
                    <div class="ic-azimuth">方位 ${data.interfered_azimuth.toFixed(1)}°</div>
                    <div class="ic-dev bad">偏差 ${data.interfered_accuracy_deg.toFixed(2)}°</div>
                </div>
            </div>
            <div class="interference-metrics">
                <div class="metric"><span class="m-label">总干扰场</span><span class="m-value">${data.total_interference_field_nT.toFixed(1)} nT</span></div>
                <div class="metric"><span class="m-label">干扰/地磁场比</span><span class="m-value">${(data.interference_ratio * 100).toFixed(1)}%</span></div>
            </div>
            <div class="interference-effects">
                <h4>各干扰源贡献</h4>
                ${effectsHtml}
            </div>
            <canvas id="interferenceChart" width="800" height="300"></canvas>
        `;

        this.renderInterferenceChart(data);
    }

    renderInterferenceChart(data) {
        const canvas = document.getElementById('interferenceChart');
        if (!canvas || !window.Chart) return;

        if (this.interferenceChart) this.interferenceChart.destroy();
        const labels = data.effects.map(e => e.display_name);
        const devContrib = data.effects.map(e => e.deviation_contribution_deg);
        const inducedField = data.effects.map(e => e.induced_field_nT);

        this.interferenceChart = new Chart(canvas.getContext('2d'), {
            type: 'bar',
            data: {
                labels,
                datasets: [
                    { label: '偏差贡献 (°)', data: devContrib, backgroundColor: 'rgba(255, 99, 132, 0.7)', yAxisID: 'y' },
                    { label: '感应场 (nT)', data: inducedField, backgroundColor: 'rgba(54, 162, 235, 0.7)', yAxisID: 'y1' },
                ]
            },
            options: {
                responsive: true,
                plugins: { title: { display: true, text: '各干扰源影响分解' } },
                scales: {
                    y: { beginAtZero: true, position: 'left', title: { display: true, text: '角度 (°)' } },
                    y1: { beginAtZero: true, position: 'right', title: { display: true, text: '场强 (nT)' }, grid: { drawOnChartArea: false } },
                }
            }
        });
    }
}

const interferenceSimulator = new InterferenceSimulator();
