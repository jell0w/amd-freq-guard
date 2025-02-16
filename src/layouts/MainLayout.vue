<script setup>
import { nextTick, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useSettingsStore } from '../stores/settings';
import Toast from 'primevue/toast';

const router = useRouter();
const settingsStore = useSettingsStore();

// onMounted(() => {
//   //使用nextTick
//   nextTick(() => {
//     Promise.all([
//       settingsStore.loadSettings(),
//       settingsStore.setupSettingsListener()
//     ]).catch(error => {
//       console.error('初始化设置失败:', error);
//     });
//   });
// })
</script>

<template>
  <div class="layout-container">

    <!-- 使用 keep-alive 包装路由视图 -->
    <router-view v-slot="{ Component }">
      <keep-alive>
        <component v-if="$route.meta.keepAlive" :is="Component" />
      </keep-alive>
      <component v-if="!$route.meta.keepAlive" :is="Component" />
    </router-view>
  </div>
</template>

<style scoped>
.layout-container {
  min-height: 100vh;
}
</style> 