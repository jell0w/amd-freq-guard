import { defineStore } from 'pinia';
import { invoker } from '../utils/invoker';
import { listen } from '@tauri-apps/api/event';
import { debounce } from 'lodash';
import { watch } from 'vue';
import toast from '../utils/toast';

let _isListenerSetup = false;
let pendingUpdates = new Set();

// 统一防抖时间
const DEFAULT_DEBOUNCE_TIME = 300;

// 创建一个防抖的更新函数，接收store参数以便回退
const debouncedUpdateSetting = debounce(async (store, key, value, oldValue) => {
  pendingUpdates.add(key);
  try {
    await invoker('update_setting', { key, value }, true);
  } catch (error) {
    console.error('设置更新失败:', error);
    // 回退到旧值，使用 $patch 避免触发 subscribe
    store.$patch({
      [key]: oldValue
    });
    toast.add({
      severity: 'error',
      summary: '设置失败',
      detail: error.toString(),
      life: 3000
    });
  } finally {
    pendingUpdates.delete(key);
  }
}, DEFAULT_DEBOUNCE_TIME);

export const useSettingsStore = defineStore('settings', {
  state: () => ({
    auto_start: false,
    auto_minimize: false,
    refresh_interval: 1000,
    frequency_threshold: 3.5,
    frequency_mode: "1",
    auto_switch_enabled: false,
    auto_switch_threshold: 25,
    trigger_action_enabled: false,
    frequency_detection_enabled: true,
    alert_debounce_seconds: 15,
    accepted_terms_of_service: -1,
  }),

  actions: {
    async loadSettings() {
      try {
        const settings = await invoker('load_settings');
        console.log('加载的设置:', settings);
        // 使用 $patch 更新状态，但不触发 subscribe
        this.$patch(settings);
      } catch (error) {
        console.error('加载设置失败:', error);
      }
    },

    async setupSettingsListener() {
      if (_isListenerSetup) return;

      // 监听后端设置变更
      await listen('settings-changed', (event) => {
        const newSettings = event.payload;
        // 只更新不在 pendingUpdates 中的值
        const filteredSettings = Object.fromEntries(
          Object.entries(newSettings).filter(([key]) => !pendingUpdates.has(key))
        );
        if (Object.keys(filteredSettings).length > 0) {
          this.$patch(filteredSettings);
        }
      });


      //改为循环注册watch
      const watchOptions = { deep: true };

      Object.keys(this.$state).forEach(key => {
        if (key.startsWith('_')) return;

        watch(
          () => this[key],
          async (newValue, oldValue) => {
            console.log("Update setting:", {key, newValue, oldValue});
            if (pendingUpdates.has(key)) return;
            debouncedUpdateSetting(this, key, newValue, oldValue);
          },
          watchOptions
        );
      });

      // // 订阅状态变更
      // this.$subscribe(
      //   (mutation, state) => {
      //     console.log('mutation', mutation, state);
      //     // 只处理 patch 类型的更新
      //     if (!mutation.type.includes('patch')) {
      //       const { newValue, oldValue, key } = mutation.events;
      //       console.log('newValue', newValue, 'oldValue', oldValue, 'key', key);

      //       if (!pendingUpdates.has(key)) {
      //         debouncedUpdateSetting(this, key, newValue, oldValue)
      //       }

      //       // Object.entries(mutation.payload).forEach(([key, value]) => {
      //       //   if (!pendingUpdates.has(key)) {
      //       //     // 保存旧值用于回退
      //       //     const oldValue = state[key];
      //       //     pendingUpdates.add(key);
      //       //     debouncedUpdateSetting(this, key, value, oldValue)
      //       //       .finally(() => {
      //       //         pendingUpdates.delete(key);
      //       //       });
      //       //   }
      //       // });
      //     }
      //   },
      //   { detached: true } // 确保组件销毁后依然能执行
      // );

      _isListenerSetup = true;
    }
  }
}); 