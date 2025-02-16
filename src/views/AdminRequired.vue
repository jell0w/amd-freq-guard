<template>
  <div class="admin-required">
    <div class="content">
      <i class="pi pi-shield" style="font-size: 3rem; color: #ff4444;"></i>
      <h1>需要管理员权限</h1>
      <p>此应用需要管理员权限才能正常运行。这是因为需要访问系统电源计划和CPU频率信息。</p>
      
      <div class="actions">
        <Button label="以管理员身份运行" 
                severity="danger"
                icon="pi pi-shield"
                @click="requestAdmin" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { invoker } from '../utils/invoker';
import Button from 'primevue/button';
// import { useToast } from 'primevue/usetoast';
import toast from '../utils/toast';

// const router = useRouter();
// const toast = useToast();

async function requestAdmin() {
  try {
    await invoker('request_admin_privileges');
  } catch (error) {
    toast.add({
      severity: 'error',
      summary: '启动失败',
      detail: error.toString(),
      life: 5000
    });
  }
}
</script>

<style scoped>
.admin-required {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  padding: 2rem;
}

.content {
  text-align: center;
  max-width: 600px;
  padding: 2rem;
  border-radius: 12px;
}

h1 {
  margin: 1rem 0;
}

p {
  line-height: 1.6;
  margin-bottom: 2rem;
}

.actions {
  display: flex;
  justify-content: center;
  gap: 1rem;
}
</style> 