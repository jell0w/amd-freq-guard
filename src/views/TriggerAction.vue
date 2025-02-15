<script setup>
import { ref, onMounted, computed } from 'vue';
import { invoker } from '../utils/invoker';
import { useRouter } from 'vue-router';
import Card from 'primevue/card';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import InputText from 'primevue/inputtext';
import Dropdown from 'primevue/dropdown';
import InputNumber from 'primevue/inputnumber';
import ToggleSwitch from 'primevue/toggleswitch';
import { v4 as uuidv4 } from 'uuid';
import { useToast } from 'primevue/usetoast';
import Message from 'primevue/message';
import SelectButton from 'primevue/selectbutton';
import { useSettingsStore } from '../stores/settings';
import { storeToRefs } from 'pinia';

const router = useRouter();
const actions = ref([]);
const powerPlans = ref([]);
const isLoading = ref(false);
const editDialog = ref(false);
const editingAction = ref(null);
// const triggerActionMasterSwitch = ref(false);
const toast = useToast();

const settingsStore = useSettingsStore();
const { trigger_action_enabled:triggerActionMasterSwitch, pending, error } = storeToRefs(settingsStore);

const triggerActionType = ref([
  { label: '计划切换', value: 'simple' },
  { label: '设置切换', value: 'setting_switch' },
  { label: '工作流', value: 'workflow' }
]);

const currentTriggerActionType = ref('simple');

// 新建/编辑动作的表单
const actionForm = ref({
  id: '',
  name: '',
  temp_plan_guid: '',
  target_plan_guid: '',
  pause_seconds: 1,
  enabled: true
});

// 加载所有触发动作
async function loadActions() {
  isLoading.value = true;
  try {
    const result = await invoker('load_trigger_actions');
    actions.value = Array.isArray(result) ? result : [];
    console.log('加载的触发动作:', actions.value); // 添加日志
  } catch (error) {
    console.error('加载触发动作失败:', error);
    actions.value = [];
  } finally {
    isLoading.value = false;
  }
}

// 加载电源计划列表
async function loadPowerPlans() {
  try {
    powerPlans.value = await invoker('get_power_plans_command');
  } catch (error) {
    console.error('加载电源计划失败:', error);
  }
}

// 加载设置
async function loadSettings() {
  try {
    const settings = await invoker('load_settings');
    triggerActionMasterSwitch.value = settings.trigger_action_enabled;
  } catch (error) {
    console.error('加载设置失败:', error);
  }
}

// 显示编辑对话框
function showEditDialog(action = null) {
  if (action) {
    actionForm.value = { ...action };
  } else {
    actionForm.value = {
      name: '',
      temp_plan_guid: '',
      target_plan_guid: '',
      pause_seconds: 1,
      enabled: false,
      version: 'simple'
    };
  }
  console.log(action);
  currentTriggerActionType.value = actionForm.value.version;
  editDialog.value = true;
}

// 获取计划名称的辅助函数
function getPlanName(guid) {
  const plan = powerPlans.value?.find(p => p.guid.toLowerCase() === guid.toLowerCase());
  return plan ? plan.name : '未知计划';
}

// 保存动作
async function saveAction() {
  if (!actionForm.value.name) {
    return;  // 添加表单验证
  }

  actionForm.value.id = uuidv4();

  try {
    // 如果当前动作要被启用，先禁用其他动作
    if (actionForm.value.enabled) {
      for (const otherAction of actions.value) {
        if (otherAction.id !== actionForm.value.id && otherAction.enabled) {
          otherAction.enabled = false;
          await invoker('save_trigger_action', { action: otherAction });
        }
      }
    }

    await invoker('save_trigger_action', {
      action: actionForm.value
    });
    editDialog.value = false;
    await loadActions();
  } catch (error) {
    console.error('保存触发动作失败:', error);
  }
}

// 删除动作
async function deleteAction(actionId) {
  try {
    await invoker('delete_trigger_action', { actionId });
    await loadActions();

    // 如果删除后没有动作了，关闭总开关
    if (!actions.value?.length) {
      triggerActionMasterSwitch.value = false;
      await saveMasterSwitch();
      toast.add({
        severity: 'info',
        summary: '触发动作处理器已关闭',
        detail: '由于没有可用的触发动作，处理器已自动关闭',
        life: 3000
      });
    }
  } catch (error) {
    console.error('删除触发动作失败:', error);
  }
}

// 添加返回函数
function handleBack() {
  router.push('/');  // 返回主页
}

// 修改保存设置函数，添加检查
async function saveMasterSwitch() {
  try {
    // 如果要开启总开关，先检查是否有动作
    if (triggerActionMasterSwitch.value && (!actions.value || actions.value.length === 0)) {
      triggerActionMasterSwitch.value = false;  // 重置开关状态
      toast.add({
        severity: 'warn',
        summary: '无法启用',
        detail: '请先创建至少一个触发动作',
        life: 3000
      });
      return;
    }

    await settingsStore.updateSetting('trigger_action_enabled', triggerActionMasterSwitch.value);
  } catch (error) {
    console.error('保存设置失败:', error);
  }
}

// 修改切换动作启用状态的函数
async function handleActionToggle(action) {
  try {
    await invoker('toggle_trigger_action', {
      actionId: action.id,
      enabled: action.enabled
    },true);
  } catch (error) {
    // 如果失败，恢复开关状态
    action.enabled = !action.enabled;
    
    // 显示错误消息
    toast.add({
      severity: 'error',
      summary: '操作失败',
      detail: error,
      life: 5000
    });
  }
}

// 直接修改值即可，watch 会自动处理保存
function toggleMasterSwitch() {
  if (!actions.value?.length && !triggerActionMasterSwitch.value) {
    toast.add({
      severity: 'warn',
      summary: '无法启用',
      detail: '请先创建至少一个触发动作',
      life: 3000
    });
    return;
  }
  
  triggerActionMasterSwitch.value = !triggerActionMasterSwitch.value;
}

onMounted(async () => {
  await settingsStore.loadSettings();
  await loadActions();
  await loadPowerPlans();
});
</script>

<template>
  <div class="trigger-action-container">
    <div class="header">
      <div class="header-left">
        <Button icon="pi pi-arrow-left" text rounded class="back-button" @click="handleBack" />
        <h1>触发动作管理</h1>
      </div>
      <div class="header-actions">
        <Button icon="pi pi-plus" label="新建动作" @click="showEditDialog()" class="new-action-button" />
      </div>
    </div>

    <!-- 修改总开关部分 -->
    <div class="global-switch">
      <div class="switch-content">
        <div class="switch-header">
          <span class="switch-title">触发动作处理器 (总开关)</span>
          <ToggleSwitch v-model="triggerActionMasterSwitch" :disabled="pending" />
        </div>
        <p class="switch-desc">
          启用后，当CPU频率超过阈值时将执行已启用的触发动作
        </p>
      </div>
    </div>

    <!-- 添加提示信息 -->
    <Message v-if="!(actions.length) > 0" severity="warn" class="empty-actions-message">
      请先创建至少一个触发动作，然后才能启用触发动作处理器
    </Message>

    <div class="actions-list">
      <Card v-for="action in actions" :key="action.id" class="action-card">
        <template #content>
          <div class="action-content">
            <div class="action-info">
              <div class="action-header">
                <h3>{{ action.name }}</h3>
                <ToggleSwitch v-model="action.enabled" :disabled="!triggerActionMasterSwitch"
                  @change="handleActionToggle(action)" />
              </div>
              <div class="action-details">
                <div class="plan-flow">
                  <span class="plan-name">{{ getPlanName(action.temp_plan_guid) }}</span>
                  <i class="pi pi-arrow-right"></i>
                  <span class="pause-time">{{ action.pause_seconds }}秒</span>
                  <i class="pi pi-arrow-right"></i>
                  <span class="plan-name">{{ getPlanName(action.target_plan_guid) }}</span>
                </div>
              </div>
            </div>
            <div class="action-actions">
              <Button icon="pi pi-pencil" text rounded @click="showEditDialog(action)" />
              <Button icon="pi pi-trash" text rounded severity="danger" @click="deleteAction(action.id)" />
            </div>
          </div>
        </template>
      </Card>
    </div>

    <Dialog v-model:visible="editDialog" :header="actionForm.id ? '编辑动作' : '新建动作'" modal :style="{
      width: '80%',
      maxWidth: '500px'
    }" class="action-dialog">
      <SelectButton v-if="!actionForm.id" v-model="currentTriggerActionType" :options="triggerActionType" optionLabel="label" optionValue="value" class="w-full" />
      <div v-if="currentTriggerActionType === 'simple'" class="action-form">
        <Message severity="secondary" v-if="!actionForm.id">在动作被触发时，先会切换到一个临时的电源计划，再等待指定时间后切换到目标计划。这么做的目的是达到一个强制刷新的效果
        </Message>
        <div class="form-field">
          <label>动作名称</label>
          <InputText v-model="actionForm.name" class="w-full" />
        </div>
        <div class="form-field">
          <label>临时计划</label>
          <Dropdown v-model="actionForm.temp_plan_guid" :options="powerPlans" optionLabel="name" optionValue="guid"
            class="w-full" />
        </div>
        <div class="form-field">
          <label>停顿时间（秒）</label>
          <InputNumber v-model="actionForm.pause_seconds" :min="1" :max="3600" />
        </div>
        <div class="form-field">
          <label>目标计划</label>
          <Dropdown v-model="actionForm.target_plan_guid" :options="powerPlans" optionLabel="name" optionValue="guid"
            class="w-full" />
        </div>
      </div>
      <div v-if="currentTriggerActionType === 'setting_switch'" class="action-form">
        <Message severity="secondary" v-if="!actionForm.id">在动作被触发时，先将指定计划里面的指定设置项的值临时修改，再等待指定时间再设置成目标值</Message>
        <Message severity="warn">这个功能还在搞搞搞，也可能咕咕咕</Message>
      </div>
      <div v-if="currentTriggerActionType === 'workflow'" class="action-form">
        <Message severity="secondary" v-if="!actionForm.id">在动作被触发时，会执行一个工作流，工作流可以包含多个动作，比如条件判断、计划切换、设置切换、脚本执行、当前运行的应用读取等</Message>
        <Message severity="warn">这个功能还在搞搞搞，但是咕咕咕的概率可能超过99%</Message>
      </div>
      <template #footer>
        <Button label="取消" text @click="editDialog = false" />
        <Button label="保存" @click="saveAction" severity="primary" />
      </template>
    </Dialog>
  </div>
</template>

<style scoped>
.trigger-action-container {
  padding: 2rem;
}

.header {
  margin-bottom: 2rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 1rem;
}

h1 {
  margin: 0;
  font-size: 1.5rem;
}

.actions-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 800px;
  margin: 0 auto;
}

.action-card {
  background: var(--card-bg);
  border: 1px solid var(--outline-color);
  box-shadow: var(--card-shadow);
  transition: all 0.2s ease;
  border-radius: 8px;
  padding: 0.75rem;
}

.action-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.action-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 2rem;
}

.action-info {
  flex: 1;
}

.action-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 0.25rem;
}

.action-header h3 {
  margin: 0;
  font-size: 1rem;
}

.action-details {
  font-size: 0.8rem;
  line-height: 1.2;
}

.plan-flow {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.pause-time {
  background: var(--section-bg);
  padding: 0.15rem 0.35rem;
  border-radius: 4px;
  font-size: 0.8rem;
}

.action-actions {
  display: flex;
  gap: 0.5rem;
}

.action-form {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 1rem 0;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.form-field label {
  font-size: 0.9rem;
}

.global-switch {
  background: var(--section-bg);
  border: 1px solid var(--outline-color);
  border-radius: 8px;
  padding: 1.5rem;
  margin-bottom: 2rem;
}

.switch-content {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.switch-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.switch-title {
  font-size: 1.1rem;
  font-weight: 500;
}

.switch-desc {
  margin: 0;
  font-size: 0.9rem;
}

.empty-actions-message {
  margin-bottom: 1rem;
  background: var(--card-bg);
  border: 1px solid var(--outline-color);
  border-radius: 8px;
  padding: 2rem;
  text-align: center;
}
</style>