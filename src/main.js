import { createApp } from "vue";
import { router } from './router'
import PrimeVue from 'primevue/config';
import Layout from "./layouts/MainLayout.vue";
import Aura from '@primevue/themes/aura';

import Tooltip from 'primevue/tooltip';

import 'primeicons/primeicons.css'


const app = createApp(Layout);
app.use(PrimeVue, {
    theme: {
        preset: Aura
    }
});
app.directive('tooltip', Tooltip);
app.use(router);
app.mount("#app");
