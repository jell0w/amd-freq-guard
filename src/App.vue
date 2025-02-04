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
import { listen } from '@tauri-apps/api/event';
import Skeleton from 'primevue/skeleton';
import { getVersion } from '@tauri-apps/api/app';

//引入lodash做防抖
import { debounce } from 'lodash';


const toast = useToast();

const cpuFrequencies = ref([]);
const autoStart = ref(false);
const autoMinimize = ref(false);
const refreshInterval = ref(1000);
const frequencyThreshold = ref(3.5);
const isRefreshing = ref(false);
const indicatorStatus = ref('normal');
const frequencyMode = ref(1);
const isLoading = ref(false);
const modeDialogVisible = ref(false);
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
const autoSwitchEnabled = ref(false);
const autoSwitchThreshold = ref(25);
const unchangedCount = ref(0);
const lastFrequencies = ref([]);
const lastUpdateCount = ref(0);
const triggerActionEnabled = ref(false);
const frequencyDetectionEnabled = ref(true);
const defaultSettings = {
  auto_start: false,
  auto_minimize: false,
  refresh_interval: 1000,
  frequency_threshold: 3.5,
  frequency_mode: "1",
  auto_switch_enabled: false,
  auto_switch_threshold: 25,
  trigger_action_enabled: false,
  frequency_detection_enabled: true,
  alert_debounce_seconds: 15
};
const triggerActions = ref([]);
const frequencyModes = [
  { label: 'SysInfo', value: 1, icon: 'pi pi-th-large', desc: '多核心检测' },
  { label: 'CalcMhz', value: 2, icon: 'pi pi-stop', desc: '主频检测' }
];

// 移除不必要的 ref
const checkTimer = ref(null);
const eventListeners = ref([]);

const alertDebounceSeconds = ref(15);

async function loadSettings() {
  try {
    const settings = await invoker('load_settings');
    console.log('加载的设置:', settings);

    // 先设置模式，这样可以确保界面正确显示
    frequencyMode.value = parseInt(settings.frequency_mode ?? defaultSettings.frequency_mode);

    // 然后设置其他值
    autoStart.value = settings.auto_start ?? defaultSettings.auto_start;
    autoMinimize.value = settings.auto_minimize ?? defaultSettings.auto_minimize;
    refreshInterval.value = settings.refresh_interval ?? defaultSettings.refresh_interval;
    frequencyThreshold.value = settings.frequency_threshold ?? defaultSettings.frequency_threshold;
    autoSwitchEnabled.value = settings.auto_switch_enabled ?? defaultSettings.auto_switch_enabled;
    autoSwitchThreshold.value = settings.auto_switch_threshold ?? defaultSettings.auto_switch_threshold;
    triggerActionEnabled.value = settings.trigger_action_enabled ?? defaultSettings.trigger_action_enabled;
    frequencyDetectionEnabled.value = settings.frequency_detection_enabled ?? defaultSettings.frequency_detection_enabled;
    alertDebounceSeconds.value = settings.alert_debounce_seconds ?? defaultSettings.alert_debounce_seconds;

    // 确保后端也使用正确的设置
    await invoker('update_monitor_settings', { settings });
  } catch (e) {
    console.error('加载设置失败:', e);
    // 使用默认值
    frequencyMode.value = parseInt(defaultSettings.frequency_mode);
    autoStart.value = defaultSettings.auto_start;
    autoMinimize.value = defaultSettings.auto_minimize;
    refreshInterval.value = defaultSettings.refresh_interval;
    frequencyThreshold.value = defaultSettings.frequency_threshold;
    autoSwitchEnabled.value = defaultSettings.auto_switch_enabled;
    autoSwitchThreshold.value = defaultSettings.auto_switch_threshold;
    triggerActionEnabled.value = defaultSettings.trigger_action_enabled;
    frequencyDetectionEnabled.value = defaultSettings.frequency_detection_enabled;
    alertDebounceSeconds.value = defaultSettings.alert_debounce_seconds;

    // 即使加载失败也要确保后端使用默认设置
    await invoker('update_monitor_settings', { settings: defaultSettings }).catch(err => {
      console.error('更新后端设置失败:', err);
    });
  }
}

async function saveSettings() {
  // console.log('保存设置');
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
      frequency_detection_enabled: frequencyDetectionEnabled.value,
      alert_debounce_seconds: alertDebounceSeconds.value,
    };
    await invoker('update_monitor_settings', { settings });
    await invoker('save_settings', { settings });
  } catch (e) {
    console.error('保存设置失败:', e);
  }
}

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
    frequencyMode.value = parseInt(mode);
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
  await loadSettings();
  await loadTriggerActions();
  await setupEventListeners();

  console.log('版本号:', await getVersion());

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

const handleIntervalChange = debounce(handleIntervalChangeReal, 200);


// 修改刷新间隔处理函数
async function handleIntervalChangeReal() {
  try {
    // 先保存设置到文件
    await saveSettings();

    // 再通知后端更新
    await invoker('update_monitor_settings', {
      settings: {
        auto_start: autoStart.value,
        auto_minimize: autoMinimize.value,
        refresh_interval: refreshInterval.value,
        frequency_threshold: frequencyThreshold.value,
        frequency_mode: String(frequencyMode.value),
        auto_switch_enabled: autoSwitchEnabled.value,
        auto_switch_threshold: autoSwitchThreshold.value,
        trigger_action_enabled: triggerActionEnabled.value,
        frequency_detection_enabled: frequencyDetectionEnabled.value,
        alert_debounce_seconds: alertDebounceSeconds.value,
      }
    });

    toast.add({
      severity: 'success',
      summary: '设置已更新',
      detail: `刷新间隔已更新为 ${refreshInterval.value} ms`,
      life: 2000
    });
  } catch (error) {
    console.error('更新刷新间隔失败:', error);
    toast.add({
      severity: 'error',
      summary: '更新失败',
      detail: '更新刷新间隔时发生错误',
      life: 3000
    });
  }
}

const handleThresholdChange = debounce(handleThresholdChangeReal, 200);

// 修改频率阈值处理函数
async function handleThresholdChangeReal() {
  try {
    await saveSettings();
    toast.add({
      severity: 'success',
      summary: '设置已更新',
      detail: `频率阈值已更新为 ${frequencyThreshold.value} GHz`,
      life: 2000
    });
  } catch (error) {
    console.error('更新频率阈值失败:', error);
    toast.add({
      severity: 'error',
      summary: '更新失败',
      detail: '更新频率阈值时发生错误',
      life: 3000
    });
  }
}

// 修改模式切换处理函数
async function handleModeChange() {
  try {
    // 先清空频率列表
    cpuFrequencies.value = [];

    // 通知后端切换模式
    await invoker('update_frequency_mode', { mode: String(frequencyMode.value) });

    // 立即执行一次频率获取
    await invoker('refresh_frequencies');

    await saveSettings();
  } catch (error) {
    console.error('切换模式失败:', error);
    toast.add({
      severity: 'error',
      summary: '切换失败',
      detail: '切换频率获取模式时发生错误',
      life: 3000
    });
  }
}

// 处理自动切换开关变化
async function handleAutoSwitchChange() {
  try {
    if (!autoSwitchEnabled.value) {
      autoSwitchThreshold.value = 0;
    } else {
      if (frequencyMode.value !== 1) {
        frequencyMode.value = 1;
        toast.add({
          severity: 'info',
          summary: '模式已切换',
          detail: '已自动切换到 SysInfo 模式以启用自动切换功能',
          life: 3000
        });
      }
      autoSwitchThreshold.value = 25;
    }

    await saveSettings();

    await invoker('update_auto_switch', {
      enabled: autoSwitchEnabled.value,
      threshold: autoSwitchThreshold.value
    });

    toast.add({
      severity: 'success',
      summary: '设置已更新',
      detail: autoSwitchEnabled.value ? '已开启自动切换' : '已关闭自动切换',
      life: 3000
    });
  } catch (error) {
    console.error('更新自动切换设置失败:', error);
    toast.add({
      severity: 'error',
      summary: '设置失败',
      detail: '更新自动切换设置时发生错误',
      life: 3000
    });
  }
}

// 修改频率检测开关处理函数
async function handleFrequencyDetectionChange() {
  try {
    await saveSettings();
  } catch (error) {
    console.error('更新频率检测设置失败:', error);
  }
}

async function checkTriggerActionStatus() {
  try {
    const hasActive = await invoker('check_active_trigger_action');
    if (!hasActive && triggerActionEnabled.value) {
      triggerActionEnabled.value = false;
      toast.add({
        severity: 'warn',
        summary: '触发动作已禁用',
        detail: '没有找到已启用的触发动作',
        life: 3000
      });
      await saveSettings();
    }
  } catch (error) {
    console.error('检查触发动作状态失败:', error);
  }
}

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
    await saveSettings();

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
    try {
      await saveSettings();
    } catch (errorOnSave) {
      // 恢复开关状态
      autoStart.value = !autoStart.value;
      toast.add({
        severity: 'error',
        summary: '设置失败',
        detail: error.toString() + ";" + errorOnSave.toString(),
        life: 5000
      });
    }




  }
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
        <h2>设置</h2>
        <div class="action-buttons">
          <Button label="触发动作管理" icon="pi pi-bolt" style="width: 100%;" @click="$router.push('/trigger-action')" />
          <Button label="电源计划管理" icon="pi pi-cog" style="width: 100%;" @click="$router.push('/power-plan')" />
        </div>
        <div class="setting-group">
          <div class="setting-group-title">主要功能开关</div>
          <div class="switch-list">
            <div class="switch-item">
              <div class="switch-header">
                <span>频率检测</span>
                <ToggleSwitch v-model="frequencyDetectionEnabled" @change="handleFrequencyDetectionChange" />
              </div>
              <p class="switch-desc">开启后将持续监控 CPU 频率变化并在超过设定的阈值时报警</p>
            </div>

            <div class="switch-item">
              <div class="switch-header">
                <span>触发动作处理器</span>
                <ToggleSwitch v-model="triggerActionEnabled" :disabled="!triggerActions.length > 0"
                  @change="saveSettings" />
              </div>
              <p class="switch-desc">当触发报警时执行已启用的触发动作</p>
              <Message v-if="!triggerActions.length > 0" severity="warn" class="switch-message">
                请先在触发动作管理中创建至少一个动作
              </Message>
            </div>
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-group-title">基本设置</div>
          <div class="setting-item">
            <span>开机自启</span>
            <ToggleSwitch v-model="autoStart" @change="handleAutoStartChange" />
          </div>
          <div class="setting-item">
            <span>自启时最小化</span>
            <ToggleSwitch v-model="autoMinimize" @change="saveSettings" />
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-group-title">监控设置</div>
          <div class="setting-item">
            <span>刷新间隔
              <i class="pi pi-question-circle" v-tooltip.top="'每隔多少毫秒刷新一次CPU频率'"
                style="cursor: help;margin: auto 0;opacity: 0.5;">
              </i>
            </span>
            <div class="interval-control">
              <Slider v-model="refreshInterval" :min="320" :max="5000" :step="10" class="custom-slider"
                @change="handleIntervalChange" />
              <InputNumber v-model="refreshInterval" :min="320" suffix=" 毫秒" @change="handleIntervalChange" />
              <Message v-if="(refreshInterval < 2000) && frequencyMode === 2" severity="warn" variant="outlined"
                size="small">过快的刷新频率可能增加CPU占用率</Message>
            </div>
          </div>

          <div class="setting-item">
            <span>频率阈值
              <i class="pi pi-question-circle" v-tooltip.top="'如果CPU频率超过此阈值，将触发报警'"
                style="cursor: help;margin: auto 0;opacity: 0.5;">
              </i>
            </span>
            <div class="interval-control">
              <Slider v-model="frequencyThreshold" :min="1.0" :max="5.0" :step="0.1" class="custom-slider"
                @change="handleThresholdChange" />
              <InputNumber v-model="frequencyThreshold" :maxFractionDigits="3" suffix=" GHz"
                @input="handleThresholdChange" />
            </div>
          </div>

          <div class="setting-item">
            <span>报警防抖时间
              <i class="pi pi-question-circle" v-tooltip.top="'如果多个报警同时触发，在防抖时间内，重复的报警行为将不起作用'"
                style="cursor: help;margin: auto 0;opacity: 0.5;">
              </i>
            </span>
            <div class="interval-control">
              <InputNumber v-model="alertDebounceSeconds" :min="5" :max="300" @change="saveSettings" suffix=" 秒"
                class="w-16" />
            </div>

          </div>
        </div>

        <div class="setting-group">
          <div class="setting-item">
            <span>频率获取模式</span>
            <div class="mode-select-container">
              <SelectButton :allowEmpty="false" v-model="frequencyMode" :options="frequencyModes" optionLabel="label"
                optionValue="value" class="frequency-mode-select" @change="handleModeChange">
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

        <!-- 添加一个pi-github图标按钮，点击后跳转到https://github.com/jell0w/amd-freq-guard -->
        <Button icon="pi pi-github" variant="text" severity="secondary"
          @click="() => openExternalLink('https://github.com/jell0w/amd-freq-guard')" />
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
            <Message v-if="frequencyMode === 1" severity="warn" class="mode-warning">
              <div class="warning-content">
                <div class="warning-text">
                  注意，在此模式下，可能在某些机型、某些条件(如此前进入了睡眠状态)下，频率会不更新，这样子的话你需要切换到CalcMhz模式。
                </div>
                <div class="auto-switch-section">
                  <div class="auto-switch-header">
                    <ToggleSwitch v-model="autoSwitchEnabled" @change="handleAutoSwitchChange" />
                    <span>自动切换到 CalcMhz 模式</span>
                  </div>
                  <div v-if="autoSwitchEnabled" class="auto-switch-details">
                    <div class="threshold-control">
                      <span>在连续</span>
                      <InputNumber v-model="autoSwitchThreshold" :min="20" :max="1000" @update:modelValue="saveSettings" />
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

.setting-item:has(> .p-toggleswitch) {
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
}

.cpu-grid {
  margin-top: 1rem;
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

.mode-help-button :deep(.p-button-icon) {
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.5);
}

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

.setting-group-title {
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.7);
  margin-bottom: 0.5rem;
  font-weight: 500;
}

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





