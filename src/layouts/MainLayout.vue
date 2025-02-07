<script setup>
import { useRouter } from 'vue-router';

import Toast from 'primevue/toast';

const router = useRouter();
</script>

<template>
  <div class="layout-container">
    <Toast />
    <!-- 使用 keep-alive 包装路由视图 -->
    <router-view v-slot="{ Component }">
      <keep-alive>
        <component 
          v-if="$route.meta.keepAlive" 
          :is="Component" 
        />
      </keep-alive>
      <component 
        v-if="!$route.meta.keepAlive" 
        :is="Component" 
      />
    </router-view>
  </div>
</template>

<style scoped>
.layout-container {
  min-height: 100vh;
}
</style> 