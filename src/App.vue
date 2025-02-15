<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import { invoker } from './utils/invoker';
import ToggleSwitch from 'primevue/toggleswitch';
import InputNumber from 'primevue/inputnumber';
import Slider from 'primevue/slider';
import Card from 'primevue/card';
import { showNotification } from './utils/native-notification';
import Button from 'primevue/button';
import SelectButton from 'primevue/selectbutton';
import Dialog from 'primevue/dialog';
import Message from 'primevue/message';
import { useToast } from 'primevue/usetoast';
import { listen } from '@tauri-apps/api/event';
import Skeleton from 'primevue/skeleton';
import { getVersion } from '@tauri-apps/api/app';
import { getGithubRepoURL } from './utils/Constants';
import { useSettingsStore } from './stores/settings';
import { storeToRefs } from 'pinia';

const toast = useToast();

const settingsStore = useSettingsStore();
const { trigger_action_enabled: triggerActionEnabled,
  frequency_threshold:frequencyThreshold,
  auto_switch_enabled:autoSwitchEnabled,
  auto_switch_threshold:autoSwitchThreshold,
  frequency_mode:frequencyMode,
  refresh_interval:refreshInterval,
  frequency_detection_enabled:frequencyDetectionEnabled,
  alert_debounce_seconds:alertDebounceSeconds,
  auto_start:autoStart,
  auto_minimize:autoMinimize,
  
 } = storeToRefs(settingsStore);

const cpuFrequencies = ref([]);

const isRefreshing = ref(false);
const indicatorStatus = ref('normal');
const isLoading = ref(false);
const modeDialogVisible = ref(false);
const hasNewVersion = ref(null);
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
const unchangedCount = ref(0);
const lastFrequencies = ref([]);
const lastUpdateCount = ref(0);
const triggerActions = ref([]);
const frequencyModes = [
  { label: 'SysInfo', value: "1", icon: 'pi pi-th-large', desc: '多核心检测' },
  { label: 'CalcMhz', value: "2", icon: 'pi pi-stop', desc: '主频检测' }
];

// 移除不必要的 ref
const checkTimer = ref(null);
const eventListeners = ref([]);

const isCheckingUpdate = ref(false);


// 修改事件监听器设置函数
async function setupEventListeners() {
  // 清理旧的监听器
  eventListeners.value.forEach(unlisten => unlisten());
  eventListeners.value = [];

  // 监听状态更新
  const stateListener = await listen('monitor-state-updated', (event) => {
    const state = event.payload;

    // 先设置刷新状态为 true
    isRefreshing.value = true;

    // 更新其他状态
    cpuFrequencies.value = state.frequencies;
    indicatorStatus.value = state.indicator_status;

    // 确保 last_update_count 的更新是即时的
    lastUpdateCount.value = state.last_update_count;

    // 如果计数为 0，确保相关状态被重置
    if (state.last_update_count === 0) {
      unchangedCount.value = 0;
      lastFrequencies.value = [...state.frequencies];
    }

    // 使用 setTimeout 而不是 requestAnimationFrame
    setTimeout(() => {
      isRefreshing.value = false;
    }, 200);
  });
  eventListeners.value.push(stateListener);

  // 监听模式切换
  const modeListener = await listen('mode-switched', (event) => {
    const { mode, auto_switch_disabled, unchanged_count } = event.payload;
    frequencyMode.value = mode;
    lastUpdateCount.value = unchanged_count || 0;  // 更新计数

    if (auto_switch_disabled) {
      autoSwitchEnabled.value = false;
      autoSwitchThreshold.value = 0;
      toast.add({
        severity: 'info',
        summary: '自动切换已禁用',
        detail: '由于频率长时间未更新，已切换到 CalcMhz 模式并禁用自动切换功能',
        life: 5000
      });
    } else {
      toast.add({
        severity: 'info',
        summary: '模式已自动切换',
        detail: '检测到频率长时间未更新，已临时切换到 CalcMhz 模式',
        life: 5000
      });
    }
  });
  eventListeners.value.push(modeListener);

  // 监听频率超限
  const freqListener = await listen('frequency-exceeded', (event) => {
    const { core, frequency, threshold } = event.payload;
    showNotification(
      '频率超限警告',
      `CPU核心 ${core + 1} 频率 (${frequency.toFixed(2)} GHz) 超过阈值 ${threshold} GHz`
    );
  });
  eventListeners.value.push(freqListener);

  // 监听指示器状态变化
  const indicatorListener = await listen('indicator-status-changed', (event) => {
    indicatorStatus.value = event.payload;
  });
  eventListeners.value.push(indicatorListener);

  // 监听模式切换事件
  const modeChangeListener = await listen('mode-changed', () => {
    // 清空频率数据
    cpuFrequencies.value = [];
    lastUpdateCount.value = 0;
  });
  eventListeners.value.push(modeChangeListener);

  // 监听阈值超过事件
  const thresholdListener = await listen('threshold-exceeded', (event) => {
    const { exceeded_count, total_cores, exceeded_cores, threshold } = event.payload;

    // 构建详细信息
    const details = exceeded_cores
      .map(core => `核心 ${core.core + 1}: ${core.frequency.toFixed(2)} GHz`)
      .join('\n');

    // 显示通知
    toast.add({
      severity: exceeded_count === total_cores ? 'error' : 'warn',
      summary: `频率超限警告 (${exceeded_count}/${total_cores})`,
      detail: `${exceeded_count} 个核心超过 ${threshold} GHz\n${details}`,
      life: 5000,
      sticky: exceeded_count === total_cores,  // 如果所有核心都超限，通知会保持显示
    });
  });
  eventListeners.value.push(thresholdListener);

  // 监听触发动作禁用事件
  const triggerActionDisabledListener = await listen('trigger-actions-disabled', (event) => {
    toast.add({
      severity: 'warn',
      summary: '触发动作已禁用',
      detail: event.payload,
      closable: true,
      sticky: true
    });
  });
  eventListeners.value.push(triggerActionDisabledListener);
}

async function loadTriggerActions() {
  try {
    const actions = await invoker('load_trigger_actions');
    triggerActions.value = Array.isArray(actions) ? actions : [];
    console.log('加载的触发动作:', triggerActions.value);
  } catch (error) {
    console.error('加载触发动作失败:', error);
    triggerActions.value = [];
  }
}

onMounted(async () => {
  console.log("data in pinia:", settingsStore.$state);
  checkUpdate(true, true, true);
  // await loadSettings();
  await loadTriggerActions();
  await setupEventListeners();

  console.log('当前版本号:', await getVersion());

  // 使用更长的间隔检查触发动作状态
  // checkTimer.value = setInterval(checkTriggerActionStatus, 10000);
});

onUnmounted(() => {
  // 清理计时器
  if (checkTimer.value) {
    clearInterval(checkTimer.value);
    checkTimer.value = null;
  }

  // 清理所有事件监听器
  eventListeners.value.forEach(unlisten => unlisten());
  eventListeners.value = [];
});

async function openExternalLink(url) {
  try {
    await invoker('open_external_link', { url });
  } catch (error) {
    console.error('打开链接失败:', error);
    toast.add({
      severity: 'error',
      summary: '打开失败',
      detail: '无法打开外部链接',
      life: 3000
    });
  }
}

async function handleAutoStartChange() {
  try {
    await invoker('toggle_autostart', { enabled: autoStart.value });

    toast.add({
      severity: 'success',
      summary: autoStart.value ? '已启用自启动' : '已禁用自启动',
      detail: autoStart.value ? '程序将在系统启动时以管理员权限自动运行' : '已取消开机自启动',
      life: 3000
    });
  } catch (error) {
    toast.add({
      severity: 'error',
      summary: '设置失败',
      detail: '更新自启动设置时发生错误，目前仅关闭开关',
      life: 5000
    });
    autoStart.value = !autoStart.value;
  }
}

async function checkUpdate(ignoreError = false, ignoreEqual = false, ignoreNewVersion = false) {
  isCheckingUpdate.value = true;
  try {
    // const jsonSchema = await invoker("get_power_plans_json_by_scheme_guid_command",{
    //   guid:"8bd00add-abf1-47cf-98b4-0e38e7999415"
    // })
    // console.log("jsonSchema",JSON.parse(jsonSchema))
    const updateInfo = await invoker('check_update');
    console.log({ updateInfo })
    if (!updateInfo.is_success) {
      if (!ignoreError) {
        toast.add({
          severity: 'error',
          summary: '检查更新失败',
          detail: updateInfo.message,
          life: 3000
        });
      }

      hasNewVersion.value = null;
      return;
    }

    if (updateInfo.has_update) {
      //为链接
      hasNewVersion.value = updateInfo.download_url;
      if (ignoreNewVersion) return
      toast.add({
        severity: 'info',
        summary: '发现新版本',
        detail: `新版本 ${updateInfo.latest_version} 已发布`,
        life: 30000,
        closable: true
      });

    } else {
      hasNewVersion.value = null;
      if (!ignoreEqual) {
        toast.add({
          severity: 'success',
          summary: '检查完成',
          detail: '当前已是最新版本',
          life: 3000
        });
      }
    }
  } catch (error) {
    hasNewVersion.value = null;
    if (!ignoreError) {
      toast.add({
        severity: 'error',
        summary: '检查更新出错',
        detail: error.toString(),
        life: 3000
      });
    }
  } finally {
    isCheckingUpdate.value = false;
  }
}

async function openGithub() {
  const url = await getGithubRepoURL();
  console.log({ url })
  await openExternalLink(url);
}

async function handleFrequencyModeChange(value) {
  //清除频率列表
  cpuFrequencies.value = [];
}

</script>

<template>
  <div class="refresh-indicator" :class="{
    'refreshing': isRefreshing,
    'warning': indicatorStatus === 'warning',
    'danger': indicatorStatus === 'danger'
  }"></div>

  <div class="container">
    <div class="app-layout">
      <div class="settings-panel">
        <div style="display: flex;align-items: center;justify-content: space-between;">
          <h2>设置</h2>
          <Button v-if="hasNewVersion" label="有版本更新" size="small" severity="secondary" variant="outlined"
            icon="pi pi-arrow-circle-up" @click="openExternalLink(hasNewVersion)" />
        </div>
        <div class="action-buttons">
          <Button label="触发动作管理" icon="pi pi-bolt" style="width: 100%;" @click="$router.push('/trigger-action')" />
          <Button label="电源计划管理" icon="pi pi-cog" style="width: 100%;" @click="$router.push('/power-plan')" />
        </div>
        <div class="setting-section">
          <h2>主要功能开关</h2>
          
          <div class="setting-subsection">
            <div class="setting-item">
              <span>频率检测</span>
              <ToggleSwitch v-model="frequencyDetectionEnabled"/>
              <p>开启后将持续监控 CPU 频率变化并在超过设定的阈值时报警</p>
            </div>
          </div>

          <div class="setting-subsection">
            <div class="setting-item">
              <span>触发动作处理器</span>
              <ToggleSwitch v-model="triggerActionEnabled" :disabled="!triggerActions.length > 0"/>
                <p>当触发报警时执行已启用的触发动作</p>
              <Message v-if="!triggerActions.length > 0" severity="warn" class="switch-message">
                请先在触发动作管理中创建至少一个动作
              </Message>
            </div>
          </div>
        </div>

        <div class="setting-section">
          <h2>基本设置</h2>
          
          <div class="setting-subsection">
            <div class="setting-item">
              <span>开机启动</span>
              <ToggleSwitch v-model="autoStart" @change="handleAutoStartChange" />
            </div>
          </div>

          <div class="setting-subsection">
            <div class="setting-item">
              <span>自启时最小化</span>
              <ToggleSwitch v-model="autoMinimize" />
            </div>
          </div>
        </div>

        <div class="setting-section">
          <div class="setting-group-title">监控设置</div>
          <div class="setting-item" style="margin-top: 1rem;">
            <span>刷新间隔
              <i class="pi pi-question-circle" v-tooltip.top="'每隔多少毫秒刷新一次CPU频率'"
                style="cursor: help;margin: auto 0;opacity: 0.5;">
              </i>
            </span>
            <div class="interval-control">
              <Slider v-model="refreshInterval" :min="320" :max="5000" :step="10" class="custom-slider"
                 />
              <InputNumber v-model="refreshInterval" :min="320" suffix=" 毫秒" />
              <Message v-if="(refreshInterval < 2000) && frequencyMode === '2'" severity="warn" variant="outlined"
                size="small">过快的刷新频率可能增加CPU占用率</Message>
            </div>
          </div>

          <div class="setting-item" style="margin-top: 1rem;">
            <span>频率阈值
              <i class="pi pi-question-circle" v-tooltip.top="'如果CPU频率超过此阈值，将触发报警'"
                style="cursor: help;margin: auto 0;opacity: 0.5;">
              </i>
            </span>
            <div class="interval-control">
              <Slider v-model="frequencyThreshold" :min="1.0" :max="5.0" :step="0.1" class="custom-slider"
                />
              <InputNumber v-model="frequencyThreshold" :maxFractionDigits="3" suffix=" GHz"
                />
            </div>
          </div>

          <div class="setting-item" style="margin-top: 1rem;">
            <span>报警防抖时间
              <i class="pi pi-question-circle" v-tooltip.top="'如果多个报警同时触发，在防抖时间内，重复的报警行为将不起作用'"
                style="cursor: help;margin: auto 0;opacity: 0.5;">
              </i>
            </span>
            <div class="interval-control">
              <InputNumber v-model="alertDebounceSeconds" :min="5" :max="300" suffix=" 秒"
                class="w-16" />
            </div>

          </div>
        </div>

        <div class="setting-section">
          <div class="setting-item">
            <span>频率获取模式</span>
            <div class="mode-select-container">
              <SelectButton :allowEmpty="false" v-model="frequencyMode" :options="frequencyModes" @update:modelValue="handleFrequencyModeChange" optionLabel="label"
                optionValue="value" class="frequency-mode-select">
                <template #option="slotProps">
                  <span v-tooltip.bottom="slotProps.option.desc" class="mode-label">
                    {{ slotProps.option.label }}
                  </span>
                </template>
              </SelectButton>
              <Button icon="pi pi-question-circle" text rounded class="mode-help-button" severity="secondary"
                @click="modeDialogVisible = true" />
            </div>
          </div>
        </div>

        <div class="setting-section">
          <div class="about-links">
            <Button icon="pi pi-refresh" severity="secondary" :loading="isCheckingUpdate" @click="checkUpdate()"
              label="检查更新" />
            <Button icon="pi pi-github" text severity="secondary" @click="openGithub" />
          </div>
        </div>
      </div>

      <div v-if="!frequencyDetectionEnabled" class="detection-disabled">
        <i class="pi pi-power-off"></i>
        <h3>频率检测已关闭</h3>
        <p>开启频率检测以监控 CPU 频率变化</p>
      </div>

      <div v-else-if="cpuFrequencies.length === 0" class="monitoring-panel">
        <Skeleton height="20px" width="120px" />
        <div class="cpu-grid">
          <Skeleton height="100px" width="120px" v-for="i in 10" :key="i" />

        </div>
      </div>

      <div v-else class="monitoring-panel">
        <h1>CPU 频率监控</h1>

        <div v-if="isLoading" class="loading-container">
          <i class="pi pi-spin pi-spinner" style="font-size: 2rem"></i>
          <span>加载中...</span>
        </div>

        <template v-if="frequencyDetectionEnabled">
          <div v-if="frequencyMode === '1'">
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
            <Message v-if="frequencyMode === '1'" severity="warn" class="mode-warning">
              <div class="warning-content">
                <div class="warning-text">
                  注意，在此模式下，可能在某些机型、某些条件(如此前进入了睡眠状态)下，频率会不更新，这样子的话你需要切换到CalcMhz模式。
                </div>
                <div class="auto-switch-section">
                  <div class="auto-switch-header">
                    <ToggleSwitch v-model="autoSwitchEnabled" />
                    <span>自动切换到 CalcMhz 模式</span>
                  </div>
                  <div v-if="autoSwitchEnabled" class="auto-switch-details">
                    <div class="threshold-control">
                      <span>在连续</span>
                      <InputNumber v-model="autoSwitchThreshold" :min="5" :max="1000"
                         />
                      <span>次未更新后切换</span>
                    </div>
                    <div v-if="lastUpdateCount > 0" class="update-status">
                      <i class="pi pi-clock"></i>
                      <span>上次数据更新是在 {{ lastUpdateCount }} 次刷新之前</span>
                    </div>
                  </div>
                </div>
              </div>
            </Message>
          </div>

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
      </div>
    </div>
  </div>

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

<style>




.container {
  height: 100vh;
  overflow: hidden;
}

.app-layout {
  display: grid;
  grid-template-columns: 320px 1fr;
  height: 100vh;
  overflow: hidden;
}

.settings-panel {
  background-color: var(--section-bg);
  padding: 1.5rem;
  /* background: rgba(0, 0, 0, 0.2); */
  border-right: 1px solid rgba(182, 182, 182, 0.1);
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  background: var(--panel-bg);
}

.monitoring-panel {
  padding: 1.5rem;
  height: 100%;
  overflow-y: auto;
}

h1 {
  font-size: 1.25rem;
  margin: 0;
}

h2 {
  font-size: 1rem;
  margin: 0;
}

.setting-group {
  background: var(--section-bg);
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
  /* margin-top: 1rem; */
}

.setting-item > span {
  font-size: 0.9rem;
}

.cpu-grid {
  margin-top: 1rem;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 0.5rem;
  overflow-y: auto;
  padding: 0.25rem;
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
  text-transform: uppercase;
}

.frequency {
  font-size: 1.25rem;
  font-weight: bold;
}

.unit {
  font-size: 0.65rem;
}

.interval-control {
  background: var(--section-bg);
  border-radius: 6px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.custom-slider {
  width: 100%;
}

.refresh-indicator {
  position: fixed;
  top: 1rem;
  right: 1rem;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #00ffcc;
  opacity: 0.2;
  transition: opacity 0.1s ease-in-out;
  z-index: 1000;
}

.refresh-indicator.refreshing {
  opacity: 0.8;
  box-shadow: 0 0 10px rgba(0, 255, 204, 0.5);
}

.refresh-indicator.warning {
  background-color: #ffaa00;
  box-shadow: 0 0 10px rgba(255, 170, 0, 0.5);
}

.refresh-indicator.danger {
  background-color: #ff4444;
  box-shadow: 0 0 10px rgba(255, 68, 68, 0.5);
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
}

.mode-label {
  font-weight: 500;
}

.mode-desc {
  font-size: 0.8rem;
  margin-left: 0.25rem;
}

.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  height: 200px;
}

.single-cpu-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  padding: 2rem;
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
  text-transform: uppercase;
}

.main-frequency {
  font-size: 3rem;
  font-weight: bold;
}

.mode-info-container {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.mode-info {
  border: 1px solid var(--outline-color);
  border-radius: 8px;
  padding: 1.5rem;
}

.mode-info h3 {
  margin: 0 0 1rem 0;
  font-size: 1.2rem;
}

.mode-description {
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
  padding-left: 1.5rem;
  position: relative;
  margin-bottom: 0.5rem;
}

.advantages li::before {
  content: '✓';
  position: absolute;
  left: 0;
}

.disadvantages li::before {
  content: '!';
  position: absolute;
  left: 0;
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
  background: var(--section-bg);
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

.update-status {
  margin-top: 0.75rem;
  padding: 0.5rem;
  border-radius: 4px;
  font-size: 0.85rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.update-status i {
  font-size: 0.9rem;
}

.setting-group-title {
  font-size: 0.9rem;
  margin-bottom: 0.5rem;
  font-weight: 500;
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

.about-links {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.setting-section {
  border: 1px solid var(--outline-color);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1rem;
}


.setting-subsection {
  background: var(--section-bg);
  border-radius: 6px;
  padding: 1rem;
  margin-top: 1rem;
}

/* CPU 频率卡片样式 */
.p-card.card-normal,
.p-card.card-exceed {
  background: var(--card-bg);
  box-shadow: var(--card-shadow);
  border: 1px solid var(--outline-color);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.p-card.card-normal:hover,
.p-card.card-exceed:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.p-card.card-exceed {
  border-color: var(--red-400);
}

/* 主频卡片样式 */
.single-cpu-container .p-card {
  background: var(--card-bg);
  box-shadow: var(--card-shadow);
  border: 1px solid var(--outline-color);
  width: 100%;
  max-width: 400px;
}

.main-frequency-info {
  text-align: center;
  padding: 2rem;
  background: var(--card-bg);
  border: 1px solid var(--outline-color);
  border-radius: 8px;
  box-shadow: var(--card-shadow);
}
</style>





