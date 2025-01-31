<script setup>
import { useRouter } from 'vue-router';

const router = useRouter();
</script>

<template>
  <div class="layout-container">
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
  background: linear-gradient(135deg, #1a1a1a, #2d2d2d);
}
</style> 