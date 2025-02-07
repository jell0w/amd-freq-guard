<script setup>
import { ref, onMounted, watch, computed } from 'vue';
import { invoker } from '../utils/invoker';
import { save, open } from '@tauri-apps/plugin-dialog';
import Card from 'primevue/card';
import Button from 'primevue/button';
import Menu from 'primevue/menu';
import { useRouter } from 'vue-router';
import Dialog from 'primevue/dialog';
import InputText from 'primevue/inputtext';
import { useToast } from 'primevue/usetoast';
import PanelMenu from 'primevue/panelmenu';
import InputNumber from 'primevue/inputnumber';
import Slider from 'primevue/slider';
import ProgressBar from 'primevue/progressbar';
import ToggleButton from 'primevue/togglebutton';
import ToggleSwitch from 'primevue/toggleswitch';
import Select from 'primevue/select';
import Message from 'primevue/message';
import SelectButton from 'primevue/selectbutton';



const toast = useToast();

const router = useRouter();
const powerPlans = ref([]);
const isLoading = ref(false);
const editDialog = ref(false);
const editingPlan = ref(null);
const newPlanName = ref('');

// 添加高级设置对话框的控制
const advancedSettingsDialog = ref(false);
const currentSettings = ref(null);
const selectedSetting = ref(null);

// 添加加载状态控制
const isLoadingSettings = ref(false);

// 添加显示隐藏设置的控制
const showHiddenSettings = ref(false);

// 添加收藏状态
const likedSettings = ref(new Set());

// 添加显示模式选项
const displayModes = ref([
  { label: '收藏的', value: 'liked', desc: '只显示已收藏的设置' },
  { label: '常规', value: 'normal', desc: '显示常规设置（不包含隐藏设置）' },
  { label: '所有', value: 'all', desc: '显示所有设置，包括隐藏设置' }
]);
const currentDisplayMode = ref('normal');

// 修改菜单项,添加高级设置选项
const menuItems = ref([
  {
    label: '重命名',
    icon: 'pi pi-pencil',
    command: () => showEditDialog()
  },
  {
    label: '高级设置',
    icon: 'pi pi-cog',
    command: () => showAdvancedSettings()
  },
  {
    label: '复制',
    icon: 'pi pi-copy',
    command: () => duplicatePlan()
  },
  {
    label: '导出',
    icon: 'pi pi-upload',
    command: () => exportPlan()
  },
  {
    label: '删除',
    icon: 'pi pi-trash',
    class: 'text-red-500',
    command: () => deletePlan()
  }
]);

// 创建一个引用来存储当前操作的计划
const selectedPlan = ref(null);
const menu = ref();

const showMenu = (event, plan) => {
  // menu.value.hide();
  selectedPlan.value = plan;
  // menu.value.show(event);
  menu.value.toggle(event);
};

// 添加编辑状态的设置
const editingSettingValue = ref(null);

// 修改选择设置的函数，添加深拷贝
function selectSetting(setting) {
  console.log('selectSetting', setting);
  selectedSetting.value = setting;
  // 深拷贝当前值，用于编辑
  editingSettingValue.value = JSON.parse(JSON.stringify(setting.current_value));
}

// 修改保存修改的函数
async function saveSettingChanges() {
  if (!selectedSetting.value || !editingSettingValue.value) return;

  try {
    // 获取当前选中的计划和设置的GUID
    const schemeGuid = selectedPlan.value.guid;
    const subgroupGuid = currentSettings.value.subgroups.find(
      sg => sg.settings.includes(selectedSetting.value)
    ).subgroup.uuid;
    const settingGuid = selectedSetting.value.setting.uuid;

    // 调用后端接口保存修改
    await invoker('write_value_set_command', {
      guid: schemeGuid,
      subgroupGuid,
      settingGuid,
      acValue: editingSettingValue.value.ac_value,
      dcValue: editingSettingValue.value.dc_value
    });

    // 更新成功后，更新显示的值
    selectedSetting.value.current_value = {
      ...editingSettingValue.value
    };
    
    // 重新加载整个电源计划的设置
    // const settingsJson = await invoker('get_power_plans_json_by_scheme_guid_command', {
    //   guid: selectedPlan.value.guid
    // });
    // currentSettings.value = JSON.parse(settingsJson);
    // settingsMenu.value = convertSettingsToMenu(currentSettings.value);

    toast.add({
      severity: 'success',
      summary: '保存成功',
      detail: '设置已更新',
      life: 3000
    });
  } catch (error) {
    console.error('保存设置失败:', error);
    toast.add({
      severity: 'error',
      summary: '保存失败',
      detail: error.toString(),
      life: 3000
    });
  }
}

// 检查是否有未保存的更改
const hasUnsavedChanges = computed(() => {
  if (!selectedSetting.value || !editingSettingValue.value) return false;
  
  const current = selectedSetting.value.current_value;
  return current.ac_value !== editingSettingValue.value.ac_value ||
         current.dc_value !== editingSettingValue.value.dc_value;
});

async function loadPowerPlans() {
  isLoading.value = true;
  try {
    const plans = await invoker('get_power_plans_command');
    powerPlans.value = plans;
    console.log(plans);
  } catch (error) {
    console.error('获取电源计划失败:', error);
  } finally {
    isLoading.value = false;
  }
}

async function setActivePlan(guid) {
  try {
    await invoker('set_active_plan_command', { guid });
    await loadPowerPlans(); // 重新加载计划列表
  } catch (error) {
    console.error('设置活动计划失败:', error);
  }
}

// 显示编辑对话框
function showEditDialog() {
  if (!selectedPlan.value) return;
  editingPlan.value = selectedPlan.value;
  newPlanName.value = selectedPlan.value.name;
  editDialog.value = true;
}

// 复制计划
async function duplicatePlan() {
  if (!selectedPlan.value) return;
  try {
    await invoker('duplicate_power_plan_command', {
      guid: selectedPlan.value.guid
    });
    await loadPowerPlans();
  } catch (error) {
    console.error('复制电源计划失败:', error);
  }
}

// 删除计划
async function deletePlan() {
  if (!selectedPlan.value) return;
  try {
    await invoker('delete_power_plan_command', {
      guid: selectedPlan.value.guid
    });
    await loadPowerPlans();
  } catch (error) {
    console.error('删除电源计划失败:', error);
  }
}

// 重命名计划
async function renamePlan() {
  if (!editingPlan.value || !newPlanName.value) return;
  try {
    await invoker('rename_power_plan_command', {
      guid: editingPlan.value.guid,
      newName: newPlanName.value
    });
    editDialog.value = false;
    await loadPowerPlans();
  } catch (error) {
    console.error('重命名电源计划失败:', error);
  }
}

// 导出电源计划
async function exportPlan() {
  if (!selectedPlan.value) return;
  try {
    const filePath = await save({
      filters: [{
        name: '电源计划文件',
        extensions: ['pow']
      }],
      defaultPath: `${selectedPlan.value.name}.pow`
    });

    if (filePath) {
      await invoker('export_power_plan_command', {
        guid: selectedPlan.value.guid,
        filePath
      });
      toast.add({
        severity: 'success',
        summary: '导出成功',
        detail: '电源计划已成功导出',
        life: 3000
      });
    }
  } catch (error) {
    console.error('导出电源计划失败:', error);
  }
}

// 导入电源计划
async function importPlan() {
  try {
    const filePath = await open({
      filters: [{
        name: '电源计划文件',
        extensions: ['pow']
      }]
    });

    if (filePath) {
      await invoker('import_power_plan_command', { filePath });
      await loadPowerPlans(); // 重新加载计划列表
      toast.add({
        severity: 'success',
        summary: '导入成功',
        detail: '电源计划已成功导入',
        life: 3000
      });
    }
  } catch (error) {
    console.error('导入电源计划失败:', error);
  }
}

// 修改数据转换函数，添加过滤逻辑
function convertSettingsToMenu(settings) {
  return settings.subgroups
    .map(subgroup => {
      // 根据不同模式过滤设置项
      const filteredSettings = subgroup.settings.filter(setting => {
        switch (currentDisplayMode.value) {
          case 'liked':
            return isSettingLiked(setting);
          case 'normal':
            return setting.attributes !== 1;
          case 'all':
            return true;
          default:
            return false;
        }
      });
      
      // 如果该分组下没有可显示的设置，则不显示该分组
      if (filteredSettings.length === 0) {
        return null;
      }

      return {
        label: subgroup.subgroup.name,
        icon: 'pi pi-folder',
        items: filteredSettings.map(setting => ({
          label: setting.setting.name,
          icon: 'pi pi-cog',
          command: () => selectSetting(setting)
        }))
      };
    })
    .filter(Boolean);
}

// 加载收藏的设置
async function loadLikedSettings() {
  try {
    const preferences = await invoker('get_liked_power_settings');
    likedSettings.value = new Set(
      preferences.liked_settings.map(item => `${item.subgroup_guid}:${item.setting_guid}`)
    );
  } catch (error) {
    console.error('加载收藏设置失败:', error);
  }
}

// 切换收藏状态
async function toggleSettingLiked(setting, isLiked) {
  try {
    const subgroupGuid = currentSettings.value.subgroups.find(
      sg => sg.settings.includes(setting)
    ).subgroup.uuid;
    
    await invoker('toggle_power_setting_liked', {
      subgroupGuid,
      settingGuid: setting.setting.uuid,
      isLiked
    });
    
    const key = `${subgroupGuid}:${setting.setting.uuid}`;
    if (isLiked) {
      likedSettings.value.add(key);
    } else {
      likedSettings.value.delete(key);
    }
  } catch (error) {
    console.error('更新收藏状态失败:', error);
    toast.add({
      severity: 'error',
      summary: '操作失败',
      detail: '更新收藏状态失败',
      life: 3000
    });
  }
}

// 检查设置是否被收藏
function isSettingLiked(setting) {
  const subgroupGuid = currentSettings.value.subgroups.find(
    sg => sg.settings.includes(setting)
  ).subgroup.uuid;
  return likedSettings.value.has(`${subgroupGuid}:${setting.setting.uuid}`);
}

// 修改加载高级设置数据的函数
async function showAdvancedSettings() {
  if (!selectedPlan.value) return;

  advancedSettingsDialog.value = true;
  isLoadingSettings.value = true;

  try {
    const settingsJson = await invoker('get_power_plans_json_by_scheme_guid_command', {
      guid: selectedPlan.value.guid
    });

    currentSettings.value = JSON.parse(settingsJson);
    settingsMenu.value = convertSettingsToMenu(currentSettings.value);
    await loadLikedSettings();
  } catch (error) {
    console.error('加载高级设置失败:', error);
    toast.add({
      severity: 'error',
      summary: '加载失败',
      detail: '加载高级设置失败',
      life: 3000
    });
  } finally {
    isLoadingSettings.value = false;
  }
}

const settingsMenu = ref([]);

const getMaxOrMinValue = (setting, type) => {
  const result = setting.possible_values.data.find(item => item.name === type);
  return result.value;
}

const clearOpenedSettingStatus = () => {
  selectedSetting.value = null;
  settingsMenu.value = []; 
}


// 监听显示隐藏设置的变化
watch(showHiddenSettings, () => {
  clearOpenedSettingStatus();
  if (currentSettings.value) {
    settingsMenu.value = convertSettingsToMenu(currentSettings.value);
  }
});

//监听高级电源设置dialog的关闭
watch(advancedSettingsDialog, () => {
  if (!advancedSettingsDialog.value) {
    clearOpenedSettingStatus();
  }
});

// 监听显示模式变化
watch(currentDisplayMode, () => {
  clearOpenedSettingStatus();
  if (currentSettings.value) {
    settingsMenu.value = convertSettingsToMenu(currentSettings.value);
  }
});

onMounted(() => {
  loadPowerPlans();
});

</script>

<template>
  <div class="power-plan-container">
    <div class="header">
      <div class="header-left">
        <Button icon="pi pi-arrow-left" text rounded class="back-button" @click="router.back()" />
        <h1>电源计划管理</h1>
      </div>
      <div class="header-actions">
        <Button icon="pi pi-download" label="导入计划" text @click="importPlan" class="import-button" />
        <Button icon="pi pi-refresh" rounded text class="refresh-button" :loading="isLoading" @click="loadPowerPlans" />
      </div>
    </div>

    <div class="plans-list">
      <Card v-for="plan in powerPlans" :key="plan.guid" :pt="{
        root: { class: ['plan-card', { active: plan.is_active }] }
      }">
        <!-- <template #header>
          <div class="card-header">
            <div class="plan-status" v-if="plan.is_active">
              <i class="pi pi-check-circle status-icon"></i>
              <span class="status-text">当前活动</span>
            </div>
          </div>
        </template> -->
        <template #content>
          <div class="plan-content">
            <div class="plan-info">
              <div class="plan-name">{{ plan.name }}</div>
              <div class="plan-guid">{{ plan.guid }}</div>
            </div>
            <div class="card-actions">
              <div v-if="plan.is_active" class="active-status">
                <i class="pi pi-check-circle"></i>
                <span>当前活动</span>
              </div>
              <template v-else>
                <Button label="设为活动" @click="setActivePlan(plan.guid)" severity="primary" size="small" />
              </template>
              <Button icon="pi pi-ellipsis-v" text rounded size="small" @click="showMenu($event, plan)"
                class="more-button" />
            </div>
          </div>
        </template>
      </Card>
    </div>

    <!-- 添加菜单 -->
    <Menu ref="menu" :model="menuItems" :popup="true" class="more-menu" />

    <!-- 重命名对话框 -->
    <Dialog v-model:visible="editDialog" modal header="重命名电源计划" :style="{ width: '30rem' }">
      <div class="rename-form">
        <InputText v-model="newPlanName" placeholder="输入新名称" class="w-full" />
      </div>
      <template #footer>
        <Button label="取消" text @click="editDialog = false" />
        <Button label="确定" @click="renamePlan" severity="primary" />
      </template>
    </Dialog>

    <!-- 修改高级设置对话框 -->
    <Dialog v-model:visible="advancedSettingsDialog" modal :header="(selectedPlan?.name||'') + ' 高级电源设置'" :style="{ width: '80vw', height: '80vh' }">
      <div class="advanced-settings-container">
        <template v-if="isLoadingSettings">
          <div class="loading-container">
            <ProgressBar mode="indeterminate" style="height: 6px;width: 90%;"></ProgressBar>
            <span>正在加载设置...</span>
          </div>
        </template>
        <template v-else>
          <!-- 左侧菜单 -->
          <div class="settings-menu">
            <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem;">
              <SelectButton v-model="currentDisplayMode" 
              :allowEmpty="false"
                           :options="displayModes" 
                           optionLabel="label"
                           optionValue="value"
                           class="display-mode-select">
                <template #option="slotProps">
                  <span v-tooltip.bottom="slotProps.option.desc" class="mode-label">
                    {{ slotProps.option.label }}
                  </span>
                </template>
              </SelectButton>
            </div>
            <PanelMenu :model="settingsMenu" class="w-full" />
          </div>

          <!-- 右侧设置区域 -->
          <div class="settings-content">
            <template v-if="selectedSetting">
              <div class="setting-header">
                <h3>{{ selectedSetting.setting.name }}</h3>
                <Button 
                  :icon="isSettingLiked(selectedSetting) ? 'pi pi-star-fill' : 'pi pi-star'" 
                  :severity="isSettingLiked(selectedSetting) ? 'warn' : 'secondary'" 
                  variant="text" 
                  rounded 
                  aria-label="Star"
                  @click="toggleSettingLiked(selectedSetting, !isSettingLiked(selectedSetting))" 
                />
              </div>

              <div class="setting-control">
                <Message severity="warn" v-if="selectedSetting.attributes === 1">
                  这是隐藏的设置，可能Windows系统不希望你修改，若你真的需要修改，请谨慎操作
                </Message>
                
                <template v-if="selectedSetting.possible_values.setting_type === 'Range'">
                  <div class="setting-field">
                    <div class="setting-field-label">
                      <label>交流电(AC)时</label>
                    </div>
                    <InputNumber v-model="editingSettingValue.ac_value"
                      :min="getMaxOrMinValue(selectedSetting, 'min')" 
                      :max="getMaxOrMinValue(selectedSetting, 'max')"
                      :step="1" 
                      :suffix="selectedSetting.possible_values.unit" />
                  </div>

                  <div class="setting-field">
                    <div class="setting-field-label">
                      <label>电池(DC)时</label>
                    </div>
                    <InputNumber v-model="editingSettingValue.dc_value"
                      :min="getMaxOrMinValue(selectedSetting, 'min')" 
                      :max="getMaxOrMinValue(selectedSetting, 'max')"
                      :step="1" 
                      :suffix="selectedSetting.possible_values.unit" />
                  </div>
                </template>

                <template v-else>
                  <div class="setting-field">
                    <div class="setting-field-label">
                      <label>交流电(AC)时</label>
                    </div>
                    <Select v-model="editingSettingValue.ac_value" 
                      :options="selectedSetting.possible_values.data"
                      optionLabel="name" 
                      optionValue="value" 
                      class="w-full" />
                  </div>

                  <div class="setting-field">
                    <div class="setting-field-label">
                      <label>电池(DC)时</label>
                    </div>
                    <Select v-model="editingSettingValue.dc_value" 
                      :options="selectedSetting.possible_values.data"
                      optionLabel="name" 
                      optionValue="value" 
                      class="w-full" />
                  </div>
                </template>

                <div class="setting-actions">
                  <Message v-if="hasUnsavedChanges">
                    你的修改需要保存才会生效，离开此页面后你的更改将会丢失
                  </Message>
                  <Button
                    label="保存修改" 
                    icon="pi pi-save"
                    severity="success"
                    @click="saveSettingChanges" />
                </div>
              </div>
            </template>

            <div v-else class="no-setting-selected">
              请从左侧选择要编辑的设置项
            </div>
          </div>
        </template>
      </div>
    </Dialog>
  </div>
</template>

<style scoped>
.power-plan-container {
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

.plans-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 800px;
  margin: 0 auto;
}

.plan-card {
  background: var(--card-bg);
  border: 1px solid var(--outline-color);
  box-shadow: var(--card-shadow);
  transition: all 0.2s ease;
  border-radius: 8px;
  padding: 0.75rem;
}

.plan-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.plan-card.active {
  border-color: var(--green-400);
  background: var(--green-50);
}

.plan-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 2rem;
}

.plan-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.plan-name {
  font-size: 1rem;
  font-weight: 500;
  line-height: 1.2;
  margin-bottom: 0.15rem;
}

.plan-guid {
  font-size: 0.65rem;
  font-family: monospace;
  opacity: 0.6;
  line-height: 1;
}

.active-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: var(--green-500);
}

.card-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.more-button {
  padding: 0.4rem !important;
}

.rename-form {
  padding: 1rem 0;
}

.settings-content {
  flex: 1;
  padding: 1rem;
  height: 100%;
  overflow-y: scroll;
}

.no-setting-selected {
  text-align: center;
  margin-top: 2rem;
  opacity: 0.6;
  padding: 2rem;
}

.loading-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
}

.settings-header {
  padding: 0 1rem 1rem 1rem;
  margin-bottom: 1rem;
  border-bottom: 1px solid var(--outline-color);
}

.settings-toggle {
  font-size: 0.9rem;
}

.display-mode-select {
  width: 100%;
}

.mode-label {
  font-size: 0.9rem;
}

/* 高级设置对话框样式 */
:deep(.p-dialog-content) {
  background: var(--card-bg);
  height: calc(80vh - 6rem); /* 减去对话框头部和底部的高度 */
  padding: 0;
  overflow: hidden; /* 防止整体滚动 */
}

.advanced-settings-container {
  display: flex;
  gap: 2rem;
  background: var(--card-bg);
  border-radius: 8px;
  height: 100%;
  padding-bottom: 1rem;
}

/* 左侧菜单样式 */
.settings-menu {
  width: 38%;
  max-width: 340px;
  min-width: 240px;
  height: 100%;
  overflow-y: scroll;
  border-right: 1px solid var(--outline-color);
  padding-right: 1rem;
  padding-left: 1rem;
  padding-top: 1rem;
}

/* 右侧内容区域样式 */
.settings-content {
  flex: 1;
  padding: 1rem;
  height: 100%;
  overflow-y: scroll;
}

/* 设置项样式 */
.setting-item {
  background: var(--section-bg);
  border-radius: 6px;
  padding: 1rem;
  margin-bottom: 1rem;
}

/* 设置组标题样式 */
.settings-group-title {
  font-size: 0.9rem;
  font-weight: 500;
  margin-bottom: 0.75rem;
  padding: 0 0.5rem;
}

/* 空状态样式 */
.empty-plans {
  background: var(--card-bg);
  border: 1px solid var(--outline-color);
  border-radius: 8px;
  padding: 2rem;
  text-align: center;
  margin-top: 2rem;
}

/* 自定义滚动条样式 */
.settings-menu::-webkit-scrollbar,
.settings-content::-webkit-scrollbar {
  width: 8px;
}

.settings-menu::-webkit-scrollbar-thumb,
.settings-content::-webkit-scrollbar-thumb {
  background: var(--outline-color);
  border-radius: 4px;
}

/* 右侧设置内容样式 */
.setting-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--outline-color);
}

.setting-header h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 500;
}

.setting-control {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 1rem;
  background: var(--section-bg);
  border-radius: 8px;
}

.setting-field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.setting-field-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
  font-weight: 500;
}

.setting-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--outline-color);
}
</style> 