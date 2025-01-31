<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import { invoker } from './utils/invoker';
import ToggleSwitch from 'primevue/toggleswitch';
import InputNumber from 'primevue/inputnumber';
import Slider from 'primevue/slider';
import Card from 'primevue/card';
import Divider from 'primevue/divider';
import { showNotification } from './utils/native-notification';
import Button from 'primevue/button';
import Dropdown from 'primevue/dropdown';
import SelectButton from 'primevue/selectbutton';
import Dialog from 'primevue/dialog';
import Message from 'primevue/message';
import { useToast } from 'primevue/usetoast';
const toast = useToast();

const cpuFrequencies = ref([]);
const autoStart = ref(false);
const autoMinimize = ref(false);
const refreshInterval = ref(1000);
const frequencyThreshold = ref(3.5);
let timer = null;

// 添加刷新状态
const isRefreshing = ref(false);
const indicatorStatus = ref('normal'); // 'normal', 'warning', 'danger'

// 添加频率模式选项
const frequencyModes = [
  { label: 'SysInfo', value: 1, icon: 'pi pi-th-large', desc: '多核心检测' },
  { label: 'CalcMhz', value: 2, icon: 'pi pi-stop', desc: '主频检测' }
];
const frequencyMode = ref(1);

// 添加加载状态
const isLoading = ref(false);

// 添加对话框控制
const modeDialogVisible = ref(false);

// 添加模式说明内容
const modeDescriptions = {
  sysinfo: {
    title: 'SysInfo 模式',
    content: '使用系统信息获取每个 CPU 核心的实时频率。这种方式可以监控所有核心的频率变化，适合需要详细了解 CPU 工作状态的场景。',
    advantages: [
      '可以监控所有核心频率',
      '较低的性能开销'
    ],
    disadvantages: [
      '可能会出现频率不更新的情况',
      '频率可能只能精确到小数点后1位',
    ],
  },
  calcmhz: {
    title: 'CalcMhz 模式',
    content: '通过计算 CPU 时钟周期来获取主频。这种方式直接测量 CPU 的实际运行频率，精度较高但只能获取整体主频。',
    advantages: [
      '测量精度高',
      '直接测量实际频率'
    ],
    disadvantages: [
      '系统资源开销较大',
      '只能获取主频，无法监控单个核心'
    ]
  }
};

// 修改自动切换相关的状态
const autoSwitchEnabled = ref(false);
const autoSwitchThreshold = ref(25);
const unchangedCount = ref(0);
const lastFrequencies = ref([]);
const lastUpdateCount = ref(0);  // 添加这行，记录上次更新是在多少次之前

// 添加新的状态
const triggerActionEnabled = ref(false);

// 添加频率检测开关状态
const frequencyDetectionEnabled = ref(true);

// 修改 Settings 接口
const defaultSettings = {
  auto_start: false,
  auto_minimize: false,
  refresh_interval: 1000,
  frequency_threshold: 3.5,
  frequency_mode: "1",
  auto_switch_enabled: false,
  auto_switch_threshold: 25
};

// 添加触发动作列表状态
const triggerActions = ref([]);

// 加载设置
async function loadSettings() {
  try {
    const settings = await invoker('load_settings');
    autoStart.value = settings.auto_start;
    autoMinimize.value = settings.auto_minimize;
    refreshInterval.value = settings.refresh_interval;
    frequencyThreshold.value = settings.frequency_threshold;
    frequencyMode.value = parseInt(settings.frequency_mode);
    autoSwitchEnabled.value = settings.auto_switch_enabled;
    autoSwitchThreshold.value = settings.auto_switch_threshold;
    triggerActionEnabled.value = settings.trigger_action_enabled;
    frequencyDetectionEnabled.value = settings.frequency_detection_enabled ?? true;  // 默认开启
  } catch (e) {
    console.error('加载设置失败:', e);
  }
}

// 保存设置
async function saveSettings() {
  try {
    const settings = {
      auto_start: autoStart.value,
      auto_minimize: autoMinimize.value,
      refresh_interval: refreshInterval.value,
      frequency_threshold: frequencyThreshold.value,
      frequency_mode: String(frequencyMode.value),
      auto_switch_enabled: autoSwitchEnabled.value,
      auto_switch_threshold: autoSwitchThreshold.value,
      trigger_action_enabled: triggerActionEnabled.value,
      frequency_detection_enabled: frequencyDetectionEnabled.value
    };
    await invoker('save_settings', { settings });
  } catch (e) {
    console.error('保存设置失败:', e);
  }
}

// 检查频率是否超过阈值
function checkFrequencyExceed(freq, index) {
  const freqGHz = freq / 1000;
  if (freqGHz > frequencyThreshold.value) {
    // 调用 Rust 函数
    invoker('trigger_clock_exceed', {
      coreId: index,
      frequency: freqGHz
    });

    // 发送通知
    showNotification(
      '频率超限警告',
      `CPU核心 ${index + 1} 频率 (${freqGHz.toFixed(2)} GHz) 超过阈值 ${frequencyThreshold.value} GHz`
    );
  }
}

// 修改频率模式变更处理
async function handleModeChange() {
  // 清空频率列表
  cpuFrequencies.value = [];
  // 保存设置
  await saveSettings();
  // 重新开始定时器
  handleIntervalChange();
}

// 修改更新CPU频率函数
async function updateCpuFrequencies() {
  // 如果频率检测关闭，则不执行
  if (!frequencyDetectionEnabled.value) {
    return;
  }

  isRefreshing.value = true;
  try {
    if (frequencyMode.value === 1) {
      const newFrequencies = await invoker("get_cpu_frequency_sysinfo");
      
      // 检查频率是否有变化
      if (autoSwitchEnabled.value && autoSwitchThreshold.value > 0) {
        const hasChanged = !lastFrequencies.value.length || 
          newFrequencies.some((freq, i) => freq !== lastFrequencies.value[i]);
        
        if (!hasChanged) {
          unchangedCount.value++;
          lastUpdateCount.value = unchangedCount.value;  // 更新计数
          if (unchangedCount.value >= autoSwitchThreshold.value) {
            // 自动切换到 CalcMhz 模式
            frequencyMode.value = 2;
            unchangedCount.value = 0;
            lastUpdateCount.value = 0;
            toast.add({
              severity: 'info',
              summary: '模式已自动切换',
              detail: '检测到频率长时间未更新，已切换到 CalcMhz 模式',
              life: 5000
            });
          }
        } else {
          unchangedCount.value = 0;
          lastUpdateCount.value = 0;
        }
        lastFrequencies.value = [...newFrequencies];
      }
      
      cpuFrequencies.value = newFrequencies;
    } else {
      cpuFrequencies.value = await invoker("get_cpu_frequency_calcmhz");
    }

    // 检查超频状态
    const exceededCount = cpuFrequencies.value.filter(freq => freq / 1000 > frequencyThreshold.value).length;
    if (exceededCount === cpuFrequencies.value.length) {
      indicatorStatus.value = 'danger';
    } else if (exceededCount > 0) {
      indicatorStatus.value = 'warning';
    } else {
      indicatorStatus.value = 'normal';
    }

    // 检查每个核心的频率
    cpuFrequencies.value.forEach((freq, index) => {
      checkFrequencyExceed(freq, index);
    });
  } catch (error) {
    console.error('获取CPU频率失败:', error);
  } finally {
    isLoading.value = false;
    setTimeout(() => {
      isRefreshing.value = false;
    }, 100);
  }
}

// 处理刷新间隔变化
function handleIntervalChange(e) {
  if (e && e.value && refreshInterval.value !== undefined) refreshInterval.value = e.value;
  // console.log('handleIntervalChange', refreshInterval.value,e);
  if (timer) {
    clearInterval(timer);
  }
  const interval = Math.max(320, refreshInterval.value);
  console.log('interval', interval);
  timer = setInterval(updateCpuFrequencies, interval);
  saveSettings();
}

// 处理自动切换开关变化
function handleAutoSwitchChange() {
  if (!autoSwitchEnabled.value) {
    autoSwitchThreshold.value = 0;
    unchangedCount.value = 0;
    lastUpdateCount.value = 0;
    lastFrequencies.value = [];
  } else {
    autoSwitchThreshold.value = 25;
  }
  saveSettings();
}

// 添加阈值变化处理函数
function handleThresholdChange() {
  saveSettings();
}

// 加载触发动作列表
async function loadTriggerActions() {
  try {
    const actions = await invoker('load_trigger_actions');
    triggerActions.value = Array.isArray(actions) ? actions : [];
    console.log('加载的触发动作:', triggerActions.value); // 添加日志
  } catch (error) {
    console.error('加载触发动作失败:', error);
    triggerActions.value = [];
  }
}

onMounted(async () => {
  await loadSettings();
  await loadTriggerActions();  // 加载触发动作列表
  updateCpuFrequencies();
  handleIntervalChange();
});

onUnmounted(() => {
  if (timer) {
    clearInterval(timer);
  }
});
</script>

<template>
  <!-- 添加回刷新指示器 -->
  <div class="refresh-indicator" :class="{
    'refreshing': isRefreshing,
    'warning': indicatorStatus === 'warning',
    'danger': indicatorStatus === 'danger'
  }"></div>

  <div class="container">
    <div class="app-layout">
      <!-- 左侧设置面板 -->
      <div class="settings-panel">
        <h2>设置</h2>
        <div class="action-buttons">
          <Button label="触发动作管理" 
                  icon="pi pi-bolt" 
                  style="width: 100%;"
                  @click="$router.push('/trigger-action')" />
          <Button label="电源计划管理" 
                  icon="pi pi-cog" 
                  style="width: 100%;"
                  @click="$router.push('/power-plan')" />
        </div>
        <div class="setting-group">
          <div class="setting-group-title">功能开关</div>
          <div class="switch-list">
            <div class="switch-item">
              <div class="switch-header">
                <span>频率检测</span>
                <ToggleSwitch v-model="frequencyDetectionEnabled"
                             @change="saveSettings" />
              </div>
              <p class="switch-desc">开启后将持续监控 CPU 频率变化</p>
            </div>

            <div class="switch-item">
              <div class="switch-header">
                <span>触发动作处理器</span>
                <ToggleSwitch v-model="triggerActionEnabled"
                             :disabled="!triggerActions.length > 0"
                             @change="saveSettings" />
              </div>
              <p class="switch-desc">当 CPU 频率超过阈值时执行已启用的触发动作</p>
              <Message v-if="!triggerActions.length > 0"
                      severity="warn"
                      class="switch-message">
                请先在触发动作管理中创建至少一个动作
              </Message>
            </div>
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-group-title">基本设置</div>
          <div class="setting-item">
            <span>开机自启</span>
            <ToggleSwitch v-model="autoStart" @change="saveSettings" />
          </div>
          <div class="setting-item">
            <span>自启时最小化</span>
            <ToggleSwitch v-model="autoMinimize" @change="saveSettings" />
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-group-title">监控设置</div>
          <div class="setting-item">
            <span>刷新间隔</span>
            <div class="interval-control">
              <Slider v-model="refreshInterval" 
                     :min="320" 
                     :max="5000" 
                     :step="10" 
                     class="custom-slider"
                     @change="handleIntervalChange" />
              <InputNumber v-model="refreshInterval" 
                          :min="320" 
                          suffix=" ms" 
                          @change="handleIntervalChange" />
            </div>
          </div>

          <div class="setting-item">
            <span>频率阈值</span>
            <div class="interval-control">
              <Slider v-model="frequencyThreshold" :min="1.0" :max="5.0" :step="0.1" class="custom-slider"
                @change="handleThresholdChange" />
              <InputNumber v-model="frequencyThreshold" :maxFractionDigits="3" suffix=" GHz" @input="handleThresholdChange" />
            </div>
          </div>
        </div>

        <div class="setting-group">
          <!-- <div class="setting-group-title">频率获取模式</div> -->
          <div class="setting-item">
            <span>频率获取模式</span>
            <div class="mode-select-container">
              <SelectButton :allowEmpty="false" v-model="frequencyMode" :options="frequencyModes" optionLabel="label" optionValue="value"
                class="frequency-mode-select" @change="handleModeChange">
                <template #option="slotProps">
                  <span v-tooltip.bottom="slotProps.option.desc" class="mode-label">{{ slotProps.option.label }}</span>
                </template>
              </SelectButton>
              <Button icon="pi pi-question-circle" text rounded class="mode-help-button" severity="secondary"
                @click="modeDialogVisible = true" />
            </div>
          </div>
        </div>
      </div>

      <!-- 右侧CPU信息面板 -->
      <div class="monitoring-panel">
        <h1>CPU 频率监控</h1>

        <!-- 加载中提示 -->
        <div v-if="isLoading" class="loading-container">
          <i class="pi pi-spin pi-spinner" style="font-size: 2rem"></i>
          <span>加载中...</span>
        </div>

        <!-- SysInfo 模式的网格布局 -->
        <template v-if="frequencyDetectionEnabled">
          <div v-if="frequencyMode === 1">
            <div class="cpu-grid">
              <Card v-for="(freq, index) in cpuFrequencies" :key="index" :pt="{
                root: { class: freq / 1000 > frequencyThreshold ? 'card-exceed' : 'card-normal' }
              }" @click="showNotification(`核心：${index + 1}`, (freq / 1000).toFixed(2))">
                <template #content>
                  <div class="core-info">
                    <div class="core-header">
                      <span class="core-label">Core {{ index + 1 }}</span>
                      <span class="unit">GHz</span>
                    </div>
                    <div class="frequency">{{ (freq / 1000).toFixed(2) }}</div>
                  </div>
                </template>
              </Card>
            </div>
            <Message v-if="frequencyMode === 1" 
                     severity="warn" 
                     class="mode-warning">
              <div class="warning-content">
                <div class="warning-text">
                  注意，在此模式下，可能在某些机型、某些条件(如此前进入了睡眠状态)下，频率会不更新，这样子的话你需要切换到CalcMhz模式。
                </div>
                <div class="auto-switch-section">
                  <div class="auto-switch-header">
                    <ToggleSwitch v-model="autoSwitchEnabled"
                                 @change="handleAutoSwitchChange" />
                    <span>自动切换到 CalcMhz 模式</span>
                  </div>
                  <div v-if="autoSwitchEnabled" 
                       class="auto-switch-details">
                    <div class="threshold-control">
                      <span>在连续</span>
                      <InputNumber v-model="autoSwitchThreshold"
                                  :min="20"
                                  :max="1000"
                                   />
                      <span>次未更新后切换</span>
                    </div>
                    <div v-if="lastUpdateCount > 0" 
                         class="update-status">
                      <i class="pi pi-clock"></i>
                      <span>上次数据更新是在 {{ lastUpdateCount }} 次刷新之前</span>
                    </div>
                  </div>
                </div>
              </div>
            </Message>
          </div>

          <!-- CalcMhz 模式的单卡片布局 -->
          <div v-else class="single-cpu-container">
            <Card v-if="cpuFrequencies.length > 0" :pt="{
              root: { class: cpuFrequencies[0] / 1000 > frequencyThreshold ? 'card-exceed' : 'card-normal' }
            }">
              <template #content>
                <div class="main-frequency-info">
                  <div class="frequency-header">
                    <span class="frequency-label">主频</span>
                    <span class="unit">GHz</span>
                  </div>
                  <div class="main-frequency">{{ (cpuFrequencies[0] / 1000).toFixed(3) }}</div>
                </div>
              </template>
            </Card>
          </div>
        </template>
        <template v-else>
          <div class="detection-disabled">
            <i class="pi pi-power-off"></i>
            <h3>频率检测已关闭</h3>
            <p>开启频率检测以监控 CPU 频率变化</p>
          </div>
        </template>
      </div>
    </div>
  </div>

  <!-- 添加模式说明对话框 -->
  <Dialog v-model:visible="modeDialogVisible" modal header="频率获取模式说明" :style="{ width: '50rem' }"
    :breakpoints="{ '1199px': '75vw', '575px': '90vw' }">
    <div class="mode-info-container">
      <div v-for="(mode, key) in modeDescriptions" :key="key" class="mode-info">
        <h3>{{ mode.title }}</h3>
        <p class="mode-description">{{ mode.content }}</p>

        <div class="features-container">
          <div class="advantages">
            <h4>优点</h4>
            <ul>
              <li v-for="(adv, index) in mode.advantages" :key="index">{{ adv }}</li>
            </ul>
          </div>

          <div class="disadvantages">
            <h4>局限性</h4>
            <ul>
              <li v-for="(dis, index) in mode.disadvantages" :key="index">{{ dis }}</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  </Dialog>
</template>

<style scoped>
.container {
  height: 100vh;
  background: linear-gradient(135deg, #1a1a1a, #2d2d2d);
  color: #fff;
  overflow: hidden;
}

.app-layout {
  display: grid;
  grid-template-columns: 320px 1fr;
  height: 100vh;
  overflow: hidden;
}

.settings-panel {
  padding: 1.5rem;
  background: rgba(0, 0, 0, 0.2);
  border-right: 1px solid rgba(255, 255, 255, 0.1);
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.monitoring-panel {
  padding: 1.5rem;
  height: 100%;
  overflow-y: auto;
}

h1 {
  font-size: 1.25rem;
  margin: 0;
  background: linear-gradient(45deg, #00ffcc, #00ccff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

h2 {
  font-size: 1rem;
  margin: 0;
  color: #fff;
}

.setting-group {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.setting-item>span {
  font-size: 0.9rem;
  color: #fff;
}

/* 只对开关类设置项使用水平布局 */
.setting-item:has(> .p-toggleswitch) {
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
}

.cpu-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 0.5rem;
  overflow-y: auto;
  padding: 0.25rem;
}

.card-normal {
  background: rgba(255, 255, 255, 0.05) !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  transition: all 0.2s ease;
}

.card-exceed {
  background: rgba(255, 50, 50, 0.15) !important;
  border: 1px solid rgba(255, 50, 50, 0.3) !important;
  transition: all 0.2s ease;
}

.card-normal:hover,
.card-exceed:hover {
  transform: translateY(-2px);
}

.core-info {
  text-align: center;
}

.core-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.core-label {
  font-size: 0.7rem;
  color: #00ffcc;
  text-transform: uppercase;
}

.frequency {
  font-size: 1.25rem;
  font-weight: bold;
  color: #fff;
  text-shadow: 0 0 10px rgba(0, 255, 204, 0.5);
}

.unit {
  font-size: 0.65rem;
  color: #888;
}

.interval-control {
  background: rgba(255, 255, 255, 0.03);
  border-radius: 6px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.custom-slider {
  width: 100%;
}

/* 确保刷新指示器样式存在 */
.refresh-indicator {
  position: fixed;
  top: 1rem;
  right: 1rem;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #00ffcc;
  opacity: 0.2;
  transition: all 0.2s ease;
  z-index: 1000;  /* 确保显示在最上层 */
}

.refresh-indicator.refreshing {
  opacity: 0.8;
}

.refresh-indicator.warning {
  background-color: #ffaa00;
}

.refresh-indicator.danger {
  background-color: #ff4444;
}

.mode-select-container {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.frequency-mode-select {
  flex: 1;
}

.mode-help-button {
  padding: 0.5rem;
  margin-left: -0.5rem;
  /* 让按钮更靠近 SelectButton */
}

.mode-help-button :deep(.p-button-icon) {
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.5);
}

/* 调整 SelectButton 右侧边框圆角 */
.frequency-mode-select :deep(.p-selectbutton .p-button:last-child) {
  border-top-right-radius: 6px;
  border-bottom-right-radius: 6px;
}

.mode-label {
  font-weight: 500;
}

.mode-desc {
  font-size: 0.8rem;
  opacity: 0.7;
  margin-left: 0.25rem;
}

.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  height: 200px;
  color: #00ffcc;
}

.single-cpu-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  padding: 2rem;
}

.single-cpu-container .card-normal,
.single-cpu-container .card-exceed {
  width: 100%;
  max-width: 400px;
}

.main-frequency-info {
  text-align: center;
  padding: 2rem;
}

.frequency-header {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.frequency-label {
  font-size: 1.2rem;
  color: #00ffcc;
  text-transform: uppercase;
}

.main-frequency {
  font-size: 3rem;
  font-weight: bold;
  color: #fff;
  text-shadow: 0 0 20px rgba(0, 255, 204, 0.5);
}

.mode-info-container {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.mode-info {
  background: rgba(0, 255, 204, 0.05);
  border: 1px solid rgba(0, 255, 204, 0.1);
  border-radius: 8px;
  padding: 1.5rem;
}

.mode-info h3 {
  color: #00ffcc;
  margin: 0 0 1rem 0;
  font-size: 1.2rem;
}

.mode-description {
  color: #fff;
  line-height: 1.6;
  margin-bottom: 1.5rem;
}

.features-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
}

.advantages h4,
.disadvantages h4 {
  color: #00ffcc;
  margin: 0 0 0.5rem 0;
  font-size: 1rem;
}

.advantages ul,
.disadvantages ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.advantages li,
.disadvantages li {
  color: #fff;
  padding-left: 1.5rem;
  position: relative;
  margin-bottom: 0.5rem;
}

.advantages li::before {
  content: '✓';
  color: #00ffcc;
  position: absolute;
  left: 0;
}

.disadvantages li::before {
  content: '!';
  color: #ffaa00;
  position: absolute;
  left: 0;
}

:deep(.p-dialog) {
  background: #1a1a1a;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

:deep(.p-dialog-header) {
  background: transparent;
  color: #00ffcc;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

:deep(.p-dialog-content) {
  background: transparent;
  color: #fff;
}

@media (max-width: 768px) {
  .app-layout {
    grid-template-columns: 1fr;
  }

  .settings-panel {
    height: auto;
  }
}

.mode-warning {
  margin-top: 1rem;
}

.warning-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.warning-text {
  line-height: 1.5;
}

.auto-switch-section {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 6px;
  padding: 1rem;
}

.auto-switch-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.auto-switch-details {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}

.threshold-control {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}

.threshold-input {
  width: 4rem;
}

.threshold-input :deep(.p-inputnumber-input) {
  height: 2rem;
  padding: 0.25rem;
  text-align: center;
  font-size: 0.9rem;
}

.update-status {
  margin-top: 0.75rem;
  padding: 0.5rem;
  background: rgba(0, 0, 0, 0.1);
  border-radius: 4px;
  font-size: 0.85rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.update-status i {
  font-size: 0.9rem;
  opacity: 0.7;
}

/* 添加设置组标题 */
.setting-group-title {
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.7);
  margin-bottom: 0.5rem;
  font-weight: 500;
}

/* 优化滑块和输入框组合 */
.interval-control {
  background: rgba(255, 255, 255, 0.03);
  border-radius: 6px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.action-buttons {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.switch-list {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.switch-item {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 6px;
  padding: 1rem;
}

.switch-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.switch-desc {
  margin: 0;
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.7);
}

.switch-message {
  margin-top: 0.75rem;
}

.detection-disabled {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.5);
  gap: 1rem;
}

.detection-disabled i {
  font-size: 3rem;
}

.detection-disabled h3 {
  margin: 0;
  font-size: 1.2rem;
}

.detection-disabled p {
  margin: 0;
  font-size: 0.9rem;
}
</style>
