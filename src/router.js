import { createRouter, createWebHashHistory } from 'vue-router'
import MainLayout from './layouts/MainLayout.vue'
import PowerPlan from './views/PowerPlan.vue'
import TriggerAction from './views/TriggerAction.vue'

const routes = [
  {
    path: '/',
    component: MainLayout,
    children: [
      {
        path: '',
        name: 'home',
        component: () => import('./App.vue'),
        meta: {
          keepAlive: true
        }
      },
      {
        path: 'power-plan',
        name: 'power-plan',
        component: PowerPlan,
        meta: {
          keepAlive: false
        }
      },
      {
        path: 'trigger-action',
        name: 'trigger-action',
        component: TriggerAction,
        meta: {
          keepAlive: false
        }
      }
    ]
  }
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes
}) 