class EraComparator {
    constructor() {
        this.crossEraChart = null;
    }

    init() {
        this.bindCrossEraEvents();
    }

    bindCrossEraEvents() {
        const btn = document.getElementById('runCrossEraBtn');
        if (btn) {
            btn.addEventListener('click', () => this.runCrossEraComparison());
        }
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
}

const eraComparator = new EraComparator();
