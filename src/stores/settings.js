import { defineStore } from 'pinia';
import { invoker } from '../utils/invoker';
import { listen } from '@tauri-apps/api/event';
import { debounce } from 'lodash';
import { watch } from 'vue';
import toast from '../utils/toast';

let _isListenerSetup = false;
let pendingUpdates = new Set();

// 定义设置项的验证规则和防抖时间
const settingConfigs = {
  frequency_threshold: {
    min: 0.5,
    max: 10.0,
    errorMsg: '频率阈值必须在 0.5-10.0 之间',
    debounceTime: 500  // 频率相关的设置使用较短的防抖
  },
  auto_switch_threshold: {
    min: 1,
    max: 100,
    errorMsg: '自动切换阈值必须在 1-100 之间',
    debounceTime: 300
  },
  refresh_interval: {
    min: 100,
    max: 10000,
    errorMsg: '刷新间隔必须在 100-10000 毫秒之间',
    debounceTime: 300
  },
  alert_debounce_seconds: {
    min: 1,
    max: 3600,
    errorMsg: '提醒防抖时间必须在 1-3600 秒之间',
    debounceTime: 300
  },
  // 开关类设置使用较短的防抖
  trigger_action_enabled: {
    debounceTime: 100
  },
  auto_switch_enabled: {
    debounceTime: 100
  },
  frequency_detection_enabled: {
    debounceTime: 100
  },
  auto_start: {
    debounceTime: 100
  },
  auto_minimize: {
    debounceTime: 100
  }
};

// 为每个设置项创建独立的防抖函数
const debouncedUpdates = {};
Object.entries(settingConfigs).forEach(([key, config]) => {
  debouncedUpdates[key] = debounce(async (store, value) => {
    await store.updateSetting(key, value);
  }, config.debounceTime);
});

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
  }),

  actions: {
    async loadSettings() {
      try {
        const settings = await invoker('load_settings');
        console.log('加载的设置:', settings);
        this.$patch(settings);
        this.setupWatchers();
      } catch (error) {
        console.error('加载设置失败:', error);
      }
    },

    // 更新单个设置
    async updateSetting(key, value) {
      console.log("Update setting:", key, value);
      if (pendingUpdates.has(key)) return;

      const oldValue = this[key].value;
      // console.log("更新设置：",key,value,"旧值：",oldValue);
      try {
        pendingUpdates.add(key);
        this.$patch({ [key]: value });
        await invoker('update_setting', { key, value },true);
      } catch (error) {
        // console.log("更新设置失败:；", error,"回退旧值：",oldValue);
        this.$patch({ [key]: oldValue });
        toast.add({
          severity: 'error',
          summary: '设置更新失败',
          detail: error.toString(),
          life: 3000
        });
        throw error;
      } finally {
        pendingUpdates.delete(key);
      }
    },

    setupWatchers() {
      const watchOptions = { deep: true };
      
      Object.keys(this.$state).forEach(key => {
        if (key.startsWith('_')) return;
        
        watch(
          () => this[key],
          async (newValue, oldValue) => {
            console.log("Update setting:", key, newValue);
            if (pendingUpdates.has(key)) return;
            
            try {
              // 前端验证
              const config = settingConfigs[key];
              if (config) {
                if (config.min !== undefined && config.max !== undefined) {
                  if (newValue < config.min || newValue > config.max) {
                    throw new Error(config.errorMsg);
                  }
                }
                
                // 使用对应设置项的防抖函数
                if (debouncedUpdates[key]) {
                  debouncedUpdates[key](this, newValue);
                } else {
                  // 如果没有特定的防抖配置，直接更新
                  await this.updateSetting(key, newValue);
                }
              } else {
                // 没有配置的设置项直接更新
                await this.updateSetting(key, newValue);
              }
            } catch (error) {
              console.error(`设置 ${key} 更新失败:`, error);
              // 验证失败时立即回滚值
              this.$patch({ [key]: oldValue });
              toast.add({
                severity: 'error',
                summary: '设置验证失败',
                detail: error.toString(),
                life: 3000
              });
            }
          },
          watchOptions
        );
      });
    },

    async setupSettingsListener() {
      if (_isListenerSetup) return;

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

      _isListenerSetup = true;
    }
  }
}); 