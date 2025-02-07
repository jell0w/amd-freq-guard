import { createApp } from "vue";
import { router } from './router'
import PrimeVue from 'primevue/config';
import Layout from "./layouts/MainLayout.vue";
import Aura from '@primevue/themes/aura';
import ToastService from 'primevue/toastservice';
import Tooltip from 'primevue/tooltip';

import 'primeicons/primeicons.css'
import './styles/colors.css'


const app = createApp(Layout);
app.use(PrimeVue, {
    theme: {
        preset: Aura,
        // options: {
        //     darkModeSelector: true,
        // }
    }
});
app.use(ToastService);
app.directive('tooltip', Tooltip);
app.use(router);
app.mount("#app");
