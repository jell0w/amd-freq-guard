<script setup>
import { ref, onMounted } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import Card from 'primevue/card';
import Button from 'primevue/button';
import { useRouter } from 'vue-router';

const router = useRouter();
const powerPlans = ref([]);

async function loadPowerPlans() {
  try {
    const plans = await invoke('get_power_plans_command');
    powerPlans.value = plans;
    console.log(plans);
  } catch (error) {
    console.error('获取电源计划失败:', error);
  }
}

async function setActivePlan(guid) {
  try {
    await invoke('set_active_plan_command', { guid });
    await loadPowerPlans(); // 重新加载计划列表
  } catch (error) {
    console.error('设置活动计划失败:', error);
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
              severity="secondary" 
              @click="loadPowerPlans" />
    </div>

    <div class="plans-grid">
      <Card v-for="plan in powerPlans" 
            :key="plan.guid"
            class="plan-card"
            :class="{ 'active': plan.is_active }">
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
              <Button label="设为活动" 
                     icon="pi pi-check"
                     severity="success"
                     :disabled="plan.is_active"
                     @click="setActivePlan(plan.guid)"
                     size="small" />
              <Button label="编辑" 
                     icon="pi pi-pencil"
                     severity="secondary"
                     text
                     size="small" />
            </div>
          </div>
        </template>
      </Card>
    </div>
  </div>
</template>

<style scoped>
.power-plan-container {
  padding: 1rem;
  min-height: 100vh;
  background: linear-gradient(135deg, #1a1a1a, #2d2d2d);
  color: #fff;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

h1 {
  font-size: 1.5rem;
  margin: 0;
  background: linear-gradient(45deg, #00ffcc, #00ccff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}

.back-button {
  color: #fff;
}

.back-button:hover {
  background: rgba(255, 255, 255, 0.1);
}

.plans-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
  padding: 0.5rem;
}

.plan-card {
  background: rgba(255, 255, 255, 0.05) !important;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  transition: all 0.3s ease;
}

.plan-card:hover {
  transform: translateY(-2px);
  border-color: rgba(255, 255, 255, 0.2) !important;
}

.plan-card.active {
  background: rgba(0, 255, 204, 0.05) !important;
  border-color: rgba(0, 255, 204, 0.2) !important;
}

.plan-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.plan-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.plan-name {
  color: #fff;
  font-size: 1.25rem;
  font-weight: 500;
  line-height: 1.2;
}

.plan-guid {
  color: #888;
  font-size: 0.7rem;
  font-family: monospace;
  opacity: 0.8;
}

.card-header {
  padding: 0.5rem;
  display: flex;
  justify-content: flex-end;
  min-height: 2.5rem;
}

.plan-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: rgba(0, 255, 204, 0.1);
  padding: 0.25rem 0.5rem;
  border-radius: 1rem;
}

.status-icon {
  color: #00ffcc;
  font-size: 0.8rem;
}

.status-text {
  color: #00ffcc;
  font-size: 0.75rem;
  font-weight: 500;
}

.card-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-start;
}
</style> 