import { defineStore } from 'pinia';
import { invoker } from '../utils/invoker';
import { listen } from '@tauri-apps/api/event';
import { debounce } from 'lodash';
import { watch } from 'vue';
import toast from '../utils/toast';

let _isListenerSetup = false;
let pendingUpdates = new Set();
// 改用 Set 来跟踪正在回滚的键
let rollbackKeys = new Set();

// 统一防抖时间
const DEFAULT_DEBOUNCE_TIME = 300;

// 修改防抖函数，在回滚时设置标志位
const debouncedUpdateSetting = debounce(async (store, key, value, oldValue) => {
  pendingUpdates.add(key);
  try {
    await invoker('update_setting', { key, value }, true);
  } catch (error) {
    console.error('设置更新失败:', error);
    // 添加到回滚集合
    rollbackKeys.add(key)
    const rollback = () => {
      store.$patch({
        [key]: oldValue
      });
    }
    rollback();
    // console.log("Rollback setting:", { key, value, oldValue });
    // console.log("Rollback keys:", JSON.stringify(rollbackKeys));
    // 从回滚集合中移除
    toast.add({
      severity: 'error',
      summary: '设置失败',
      detail: error.toString(),
      life: 3000
    });
  } finally {
    pendingUpdates.delete(key);
    if (rollbackKeys.has(key)) {
      //设置一个延迟
      setTimeout(() => {
        if(rollbackKeys.has(key))rollbackKeys.delete(key);
      }, DEFAULT_DEBOUNCE_TIME);
    }
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

        // 在 watch 中检查标志位
        watch(
          () => this[key],
          async (newValue, oldValue) => {
            // 检查这个具体的 key 是否在回滚中
            if (rollbackKeys.has(key)) {
              rollbackKeys.delete(key);
              return
            }

            if (pendingUpdates.has(key)) return;
            console.log("Update setting:", { key, newValue, oldValue });
            debouncedUpdateSetting(this, key, newValue, oldValue);
          },
          watchOptions
        );
      });

      _isListenerSetup = true;
    }
  }
}); 