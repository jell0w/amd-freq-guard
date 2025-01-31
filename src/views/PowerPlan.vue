<script setup>
import { ref, onMounted } from 'vue';
import { invoker } from '../utils/invoker';
import Card from 'primevue/card';
import Button from 'primevue/button';
import Menu from 'primevue/menu';
import { useRouter } from 'vue-router';
import Dialog from 'primevue/dialog';
import InputText from 'primevue/inputtext';

const router = useRouter();
const powerPlans = ref([]);
const isLoading = ref(false);
const editDialog = ref(false);
const editingPlan = ref(null);
const newPlanName = ref('');

// 为每个计划创建一个菜单
const menuItems = ref([
  {
    label: '重命名',
    icon: 'pi pi-pencil',
    command: () => showEditDialog()
  },
  {
    label: '复制',
    icon: 'pi pi-copy',
    command: () => duplicatePlan()
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
  selectedPlan.value = plan;
  menu.value.show(event);
};

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

onMounted(() => {
  loadPowerPlans();
});
</script>

<template>
  <div class="power-plan-container">
    <div class="header">
      <div class="header-left">
        <Button icon="pi pi-arrow-left"
                text
                rounded
                class="back-button"
                @click="router.back()" />
        <h1>电源计划管理</h1>
      </div>
      <Button icon="pi pi-refresh" 
              rounded 
              text 
              class="refresh-button"
              :loading="isLoading"
              @click="loadPowerPlans" />
    </div>

    <div class="plans-list">
      <Card v-for="plan in powerPlans" 
            :key="plan.guid"
            :pt="{
              root: { class: ['plan-card', { active: plan.is_active }] }
            }">
        <template #header>
          <div class="card-header">
            <div class="plan-status" v-if="plan.is_active">
              <i class="pi pi-check-circle status-icon"></i>
              <span class="status-text">当前活动</span>
            </div>
          </div>
        </template>
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
                <Button label="设为活动" 
                       @click="setActivePlan(plan.guid)"
                       severity="primary"
                       size="small" />
              </template>
              <Button icon="pi pi-ellipsis-v"
                     text
                     rounded
                     size="small"
                     @click="showMenu($event, plan)"
                     class="more-button" />
            </div>
          </div>
        </template>
      </Card>
    </div>

    <!-- 添加菜单 -->
    <Menu ref="menu" 
          :model="menuItems"
          :popup="true"
          class="more-menu" />

    <!-- 重命名对话框 -->
    <Dialog v-model:visible="editDialog"
            modal
            header="重命名电源计划"
            :style="{ width: '30rem' }">
      <div class="rename-form">
        <InputText v-model="newPlanName"
                   placeholder="输入新名称"
                   class="w-full" />
      </div>
      <template #footer>
        <Button label="取消"
                text
                @click="editDialog = false" />
        <Button label="确定"
                @click="renamePlan"
                severity="primary" />
      </template>
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

.back-button {
  color: #fff;
}

h1 {
  margin: 0;
  font-size: 1.5rem;
  color: #fff;
}

.plans-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 800px;
  margin: 0 auto;
}

.plan-card {
  background: rgba(255, 255, 255, 0.05) !important;
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  transition: all 0.2s ease;
}

.plan-card:hover {
  transform: translateY(-2px);
  background: rgba(255, 255, 255, 0.08) !important;
}

.plan-card.active {
  background: rgba(0, 255, 204, 0.05) !important;
  border-color: rgba(0, 255, 204, 0.2) !important;
}

.card-header {
  display: none;
}

.plan-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 2.5rem;
}

.plan-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.plan-name {
  color: #fff;
  font-size: 1rem;
  font-weight: 500;
  line-height: 1.2;
}

.plan-guid {
  color: #888;
  font-size: 0.65rem;
  font-family: monospace;
}

.active-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: #00ffcc;
  font-size: 0.8rem;
}

.active-status i {
  font-size: 0.9rem;
}

.card-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.card-actions :deep(.p-button) {
  padding: 0.4rem 0.8rem;
}

.card-actions :deep(.p-button-label) {
  font-size: 0.8rem;
}

.refresh-button {
  color: #fff;
}

.refresh-button:hover {
  background: rgba(255, 255, 255, 0.1);
}

.more-button {
  padding: 0.4rem !important;
}

.more-button:hover {
  background: rgba(255, 255, 255, 0.1) !important;
}

:deep(.more-menu) {
  background: #1a1a1a;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  padding: 0.25rem;
}

:deep(.more-menu .p-menuitem-link) {
  color: #fff;
  padding: 0.5rem 0.75rem;
  border-radius: 4px;
}

:deep(.more-menu .p-menuitem-link:hover) {
  background: rgba(255, 255, 255, 0.1);
}

:deep(.more-menu .p-menuitem-link .p-menuitem-icon) {
  color: #888;
}

:deep(.more-menu .p-menuitem-link:hover .p-menuitem-icon) {
  color: #00ffcc;
}

:deep(.more-menu .text-red-500 .p-menuitem-icon),
:deep(.more-menu .text-red-500:hover .p-menuitem-icon) {
  color: #ff4444;
}

.rename-form {
  padding: 1rem 0;
}

:deep(.p-dialog-content) {
  padding-bottom: 0;
}

:deep(.p-dialog-footer) {
  padding: 1rem 1.5rem;
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style> 