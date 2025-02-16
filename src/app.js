import { createApp } from "vue";
import { router } from './router'
import PrimeVue from 'primevue/config';
import Layout from "./layouts/MainLayout.vue";
import Aura from '@primevue/themes/aura';
import ToastService from 'primevue/toastservice';
import Tooltip from 'primevue/tooltip';
import { createPinia } from 'pinia';
// import {EventPlugin} from "@pinia/events"
import { useSettingsStore } from './stores/settings';

import 'primeicons/primeicons.css'
import './styles/colors.css'

const app = createApp(Layout);
const pinia = createPinia();
app.use(pinia);
app.use(PrimeVue, {
    theme: {
        preset: Aura,
        // options: {
        //     darkModeSelector: true,
        // }
    }
});
const settingsStore = useSettingsStore();
Promise.all([
    settingsStore.loadSettings(),
    settingsStore.setupSettingsListener()
  ]).catch(error => {
    console.error('初始化设置失败:', error);
  });

app.use(ToastService);
app.directive('tooltip', Tooltip);
app.use(router);

export default app;
