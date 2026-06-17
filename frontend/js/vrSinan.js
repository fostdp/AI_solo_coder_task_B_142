class VRSinan {
    constructor() {
        this.interactive3D = null;
    }

    init() {
        this.bindInteractiveEvents();
        this.initInteractive3D();
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

const vrSinan = new VRSinan();

async function initAllExperienceModules() {
    try {
        const [devicesRes, interferencesRes] = await Promise.all([
            dataService.getDeviceTypes(),
            dataService.getInterferenceTypes(),
        ]);
        const deviceTypes = devicesRes.devices || [];
        const interferenceTypes = interferencesRes.interference_types || [];

        if (window.deviceCompare) deviceCompare.init(deviceTypes);
        if (window.eraComparator) eraComparator.init();
        if (window.interferenceSimulator) interferenceSimulator.init(interferenceTypes);
        if (window.vrSinan) vrSinan.init();
    } catch (e) {
        console.warn('加载元数据失败:', e);
        if (window.deviceCompare) deviceCompare.init([]);
        if (window.eraComparator) eraComparator.init();
        if (window.interferenceSimulator) interferenceSimulator.init([]);
        if (window.vrSinan) vrSinan.init();
    }
}

document.addEventListener('DOMContentLoaded', () => {
    setTimeout(() => initAllExperienceModules(), 300);
});
