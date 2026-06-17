class ExperienceFeatures {
    constructor() {
        this.deviceTypes = [];
        this.interferenceTypes = [];
        this.interactive3D = null;
        this.compareChart = null;
        this.crossEraChart = null;
        this.interferenceChart = null;
    }

    async init() {
        try {
            const [devicesRes, interferencesRes] = await Promise.all([
                dataService.getDeviceTypes(),
                dataService.getInterferenceTypes(),
            ]);
            this.deviceTypes = devicesRes.devices || [];
            this.interferenceTypes = interferencesRes.interference_types || [];
        } catch (e) {
            console.warn('加载元数据失败:', e);
        }

        this.renderDeviceCheckboxes();
        this.renderInterferenceTypeOptions();
        this.bindCompareEvents();
        this.bindCrossEraEvents();
        this.bindInterferenceEvents();
        this.bindInteractiveEvents();
        this.initInteractive3D();
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

    bindCompareEvents() {
        const btn = document.getElementById('runCompareBtn');
        if (btn) {
            btn.addEventListener('click', () => this.runDeviceComparison());
        }
    }

    bindCrossEraEvents() {
        const btn = document.getElementById('runCrossEraBtn');
        if (btn) {
            btn.addEventListener('click', () => this.runCrossEraComparison());
        }
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

    bindInteractiveEvents() {
        const sliders = [
            { id: 'intYear', display: 'intYearValue', fmt: v => v },
            { id: 'intMoment', display: 'intMomentValue', fmt: v => parseFloat(v).toFixed(3) },
            { id: 'intRemanence', display: 'intRemanenceValue', fmt: v => parseFloat(v).toFixed(2) },
            { id: 'intTemp', display: 'intTempValue', fmt: v => v },
            { id: 'intFriction', display: 'intFrictionValue', fmt: v => parseFloat(v).toFixed(2) },
            { id: 'intAnisotropy', display: 'intAnisotropyValue', fmt: v => v },
            { id: 'intLength', display: 'intLengthValue', fmt: v => parseFloat(v).toFixed(1) },
            { id: 'intWidth', display: 'intWidthValue', fmt: v => parseFloat(v).toFixed(1) },
            { id: 'intThickness', display: 'intThicknessValue', fmt: v => parseFloat(v).toFixed(1) },
        ];

        sliders.forEach(s => {
            const slider = document.getElementById(s.id);
            const display = document.getElementById(s.display);
            if (slider && display) {
                slider.addEventListener('input', () => {
                    display.textContent = s.fmt(slider.value);
                });
                slider.addEventListener('change', () => {
                    display.textContent = s.fmt(slider.value);
                });
            }
        });

        const runBtn = document.getElementById('runInteractiveBtn');
        if (runBtn) {
            runBtn.addEventListener('click', () => this.runInteractiveSimulation());
        }

        const deviceType = document.getElementById('intDeviceType');
        if (deviceType) {
            deviceType.addEventListener('change', () => {
                this.adjustDefaultsForDevice(deviceType.value);
            });
        }
    }

    adjustDefaultsForDevice(type) {
        const defaults = {
            sinan:      { moment: 0.05, remanence: 0.4, friction: 0.1,  length: 17, width: 8,  thickness: 15 },
            zhinanyu:   { moment: 0.002, remanence: 0.3, friction: 0.005, length: 6, width: 1.5, thickness: 2 },
            han_luopan: { moment: 0.0005, remanence: 0.6, friction: 0.02, length: 3, width: 0.2, thickness: 0.1 },
            mems_compass: { moment: 0.0, remanence: 0.0, friction: 0.0, length: 0.2, width: 0.2, thickness: 0.05 },
        };
        const d = defaults[type] || defaults.sinan;

        const setSlider = (id, value, displayId, fmt) => {
            const el = document.getElementById(id);
            const disp = document.getElementById(displayId);
            if (el) { el.value = value; }
            if (disp) { disp.textContent = fmt(value); }
        };
        setSlider('intMoment', d.moment, 'intMomentValue', v => parseFloat(v).toFixed(3));
        setSlider('intRemanence', d.remanence, 'intRemanenceValue', v => parseFloat(v).toFixed(2));
        setSlider('intFriction', d.friction, 'intFrictionValue', v => parseFloat(v).toFixed(2));
        setSlider('intLength', d.length, 'intLengthValue', v => parseFloat(v).toFixed(1));
        setSlider('intWidth', d.width, 'intWidthValue', v => parseFloat(v).toFixed(1));
        setSlider('intThickness', d.thickness, 'intThicknessValue', v => parseFloat(v).toFixed(1));
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

    async runCrossEraComparison() {
        const lat = parseFloat(document.getElementById('ceLat').value);
        const lon = parseFloat(document.getElementById('ceLon').value);
        const ancientYear = parseFloat(document.getElementById('ceAncientYear').value);
        const modernYear = parseFloat(document.getElementById('ceModernYear').value);
        const ancientDevice = document.getElementById('ceAncientDevice').value;
        const temperature = parseFloat(document.getElementById('ceTemp').value);
        const expectedAzimuth = parseFloat(document.getElementById('ceAzimuth').value);

        showToast('正在运行跨时代对比...');
        try {
            const resp = await dataService.compareCrossEra({
                location_lat: lat,
                location_lon: lon,
                ancient_year: ancientYear,
                modern_year: modernYear,
                ancient_device: ancientDevice,
                temperature,
                expected_azimuth: expectedAzimuth,
            });
            this.renderCrossEraResults(resp.data);
            showToast('跨时代对比完成', 'success');
        } catch (e) {
            showToast('跨时代对比失败: ' + e.message, 'error');
        }
    }

    renderCrossEraResults(data) {
        const container = document.getElementById('crossEraResults');
        if (!container) return;

        const improvementClass = data.improvement_factor >= 10 ? 'improvement-huge'
            : data.improvement_factor >= 5 ? 'improvement-large' : 'improvement-moderate';

        container.innerHTML = `
            <div class="cross-era-header">
                <div class="era-panel ancient">
                    <h4>古代 ${data.ancient.display_name}</h4>
                    <p class="era-years">${data.ancient.era}</p>
                    <div class="era-accuracy">
                        <span class="accuracy-label">平均偏差</span>
                        <span class="accuracy-value large">${data.ancient.mean_deviation_deg.toFixed(2)}°</span>
                    </div>
                    <p class="era-note">${data.ancient.notes}</p>
                </div>
                <div class="era-arrow ${improvementClass}">
                    <div class="arrow-symbol">⏩</div>
                    <div class="improvement-text">精度提升</div>
                    <div class="improvement-value">×${data.improvement_factor.toFixed(0)}</div>
                    <div class="gap-value">误差缩小 ${data.accuracy_gap.toFixed(1)}°</div>
                </div>
                <div class="era-panel modern">
                    <h4>现代 ${data.modern_mems.display_name}</h4>
                    <p class="era-years">${data.modern_mems.era}</p>
                    <div class="era-accuracy">
                        <span class="accuracy-label">平均偏差</span>
                        <span class="accuracy-value large best">${data.modern_mems.mean_deviation_deg.toFixed(2)}°</span>
                    </div>
                    <p class="era-note">${data.modern_mems.notes}</p>
                </div>
            </div>
            <div class="cross-era-narrative">
                <h4>技术叙事</h4>
                <p>${data.narrative}</p>
            </div>
            <div class="cross-era-context">
                <h4>历史背景</h4>
                <p>${data.historical_context}</p>
            </div>
            <canvas id="crossEraChart" width="800" height="300"></canvas>
        `;

        this.renderCrossEraChart(data);
    }

    renderCrossEraChart(data) {
        const canvas = document.getElementById('crossEraChart');
        if (!canvas || !window.Chart) return;

        if (this.crossEraChart) this.crossEraChart.destroy();
        this.crossEraChart = new Chart(canvas.getContext('2d'), {
            type: 'bar',
            data: {
                labels: [`古代 ${data.ancient.display_name}`, '现代 MEMS 电子罗盘'],
                datasets: [
                    {
                        label: '平均偏差 (°)',
                        data: [data.ancient.mean_deviation_deg, data.modern_mems.mean_deviation_deg],
                        backgroundColor: ['rgba(139, 90, 43, 0.7)', 'rgba(0, 150, 136, 0.7)'],
                        borderColor: ['#8b5a2b', '#009688'],
                        borderWidth: 2,
                    }
                ]
            },
            options: {
                indexAxis: 'y',
                responsive: true,
                plugins: { title: { display: true, text: '跨时代精度差距（越低越好）' } },
                scales: { x: { beginAtZero: true, title: { display: true, text: '平均偏差 (°)' } } }
            }
        });
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

    initInteractive3D() {
        const canvas = document.getElementById('interactiveCanvas');
        if (!canvas || !window.THREE) return;

        const width = canvas.parentElement.clientWidth || 600;
        const height = 500;
        canvas.width = width;
        canvas.height = height;

        const scene = new THREE.Scene();
        scene.background = new THREE.Color(0xf5f0e1);

        const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
        camera.position.set(0, 1.5, 3);
        camera.lookAt(0, 0, 0);

        const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
        renderer.setSize(width, height);
        renderer.shadowMap.enabled = true;

        scene.add(new THREE.AmbientLight(0xffffff, 0.6));
        const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
        dirLight.position.set(3, 5, 2);
        dirLight.castShadow = true;
        scene.add(dirLight);

        const compassGroup = new THREE.Group();
        scene.add(compassGroup);

        const plateGeo = new THREE.CylinderGeometry(1.2, 1.2, 0.05, 64);
        const plateMat = new THREE.MeshStandardMaterial({ color: 0xc9a66b, roughness: 0.7, metalness: 0.3 });
        const plate = new THREE.Mesh(plateGeo, plateMat);
        plate.position.y = -0.1;
        plate.receiveShadow = true;
        compassGroup.add(plate);

        const directions = [
            { name: '北', angle: 0, color: 0xff0000 },
            { name: '东', angle: Math.PI / 2, color: 0x333333 },
            { name: '南', angle: Math.PI, color: 0x333333 },
            { name: '西', angle: -Math.PI / 2, color: 0x333333 },
        ];
        directions.forEach(d => {
            const markGeo = new THREE.CylinderGeometry(0.03, 0.03, 0.08, 16);
            const markMat = new THREE.MeshStandardMaterial({ color: d.color });
            const mark = new THREE.Mesh(markGeo, markMat);
            mark.position.set(Math.sin(d.angle) * 0.95, 0, Math.cos(d.angle) * 0.95);
            compassGroup.add(mark);
        });

        const spoonGroup = new THREE.Group();

        const handleGeo = new THREE.CylinderGeometry(0.04, 0.06, 0.9, 16);
        const spoonMat = new THREE.MeshStandardMaterial({ color: 0x2b2b2b, roughness: 0.5, metalness: 0.6 });
        const handle = new THREE.Mesh(handleGeo, spoonMat);
        handle.rotation.z = Math.PI / 2;
        handle.position.x = 0.2;
        handle.castShadow = true;
        spoonGroup.add(handle);

        const bowlGeo = new THREE.SphereGeometry(0.28, 32, 32, 0, Math.PI * 2, Math.PI / 2, Math.PI / 2);
        const bowl = new THREE.Mesh(bowlGeo, spoonMat);
        bowl.position.x = -0.3;
        bowl.rotation.y = Math.PI / 2;
        bowl.castShadow = true;
        spoonGroup.add(bowl);

        compassGroup.add(spoonGroup);

        if (window.THREE.OrbitControls) {
            new THREE.OrbitControls(camera, renderer.domElement);
        }

        this.interactive3D = { scene, camera, renderer, spoonGroup, compassGroup };

        const animate = () => {
            requestAnimationFrame(animate);
            renderer.render(scene, camera);
        };
        animate();
    }

    async runInteractiveSimulation() {
        const deviceType = document.getElementById('intDeviceType').value;
        const targetYear = parseFloat(document.getElementById('intYear').value);
        const lat = 34.265;
        const lon = 108.955;
        const moment = parseFloat(document.getElementById('intMoment').value);
        const remanence = parseFloat(document.getElementById('intRemanence').value);
        const temperature = parseFloat(document.getElementById('intTemp').value);
        const friction = parseFloat(document.getElementById('intFriction').value);
        const anisotropy = parseFloat(document.getElementById('intAnisotropy').value);
        const lengthM = parseFloat(document.getElementById('intLength').value) / 100;
        const widthM = parseFloat(document.getElementById('intWidth').value) / 100;
        const thicknessM = parseFloat(document.getElementById('intThickness').value) / 1000;

        const req = {
            device_type: deviceType,
            target_year: targetYear,
            location_lat: lat,
            location_lon: lon,
            magnetic_moment_magnitude: moment,
            remanence,
            temperature,
            friction_coefficient: friction,
            anisotropy_constant: anisotropy,
            spoon_length_m: lengthM,
            spoon_width_m: widthM,
            spoon_thickness_m: thicknessM,
            expected_azimuth: 180,
        };

        try {
            const resp = await dataService.simulateInteractive(req);
            this.updateInteractiveDisplay(resp.data);
            showToast('交互式仿真完成', 'success');
        } catch (e) {
            showToast('交互式仿真失败: ' + e.message, 'error');
        }
    }

    updateInteractiveDisplay(data) {
        document.getElementById('intSimAzimuth').textContent = data.simulated_azimuth.toFixed(2) + '°';
        document.getElementById('intAccuracy').textContent = data.pointing_accuracy_deg.toFixed(2) + '°';
        document.getElementById('intFieldIntensity').textContent = data.geomagnetic_intensity_nT.toFixed(0) + ' nT';
        document.getElementById('intEffMoment').textContent = data.effective_moment_magnitude.toFixed(4) + ' A·m²';
        document.getElementById('intThermal').textContent = data.thermal_fluctuation_deg.toFixed(2) + '°';

        if (this.interactive3D && this.interactive3D.spoonGroup) {
            const target = (data.simulated_azimuth - 180) * Math.PI / 180;
            this.interactive3D.spoonGroup.rotation.y = target;
        }

        const insightsContainer = document.getElementById('interactiveInsights');
        if (insightsContainer) {
            insightsContainer.innerHTML = `
                <h4>🔬 物理洞察</h4>
                <ul class="insight-list">
                    ${data.physics_insights.map(i => `<li>${i}</li>`).join('')}
                </ul>
                <div class="demag-info">
                    <h5>退磁张量</h5>
                    <table class="demag-table">
                        <tr><td>Nxx</td><td>${data.demagnetization_tensor.n_xx?.toFixed(4) || '-'}</td>
                            <td>Nyy</td><td>${data.demagnetization_tensor.n_yy?.toFixed(4) || '-'}</td>
                            <td>Nzz</td><td>${data.demagnetization_tensor.n_zz?.toFixed(4) || '-'}</td></tr>
                        <tr><td>Nxy</td><td>${data.demagnetization_tensor.n_xy?.toFixed(4) || '-'}</td>
                            <td>Nxz</td><td>${data.demagnetization_tensor.n_xz?.toFixed(4) || '-'}</td>
                            <td>Nyz</td><td>${data.demagnetization_tensor.n_yz?.toFixed(4) || '-'}</td></tr>
                    </table>
                </div>
            `;
        }
    }
}

const experienceFeatures = new ExperienceFeatures();

document.addEventListener('DOMContentLoaded', () => {
    setTimeout(() => experienceFeatures.init(), 300);
});
