import { createRouter, createWebHashHistory } from 'vue-router'
import { ref } from 'vue'
import { invoker } from './utils/invoker'
import App from './App.vue'
import AdminRequired from './views/AdminRequired.vue'
import MainLayout from './layouts/MainLayout.vue'
import PowerPlan from './views/PowerPlan.vue'
import TriggerAction from './views/TriggerAction.vue'

// 创建一个响应式变量来存储管理员权限状态
export const hasAdminPrivileges = ref(false)

const routes = [
  {
    path: '/admin-required',
    name: 'AdminRequired',
    component: AdminRequired
  },
  {
    path: '/',
    name: 'Home',
    component: App,
    meta: { requiresAdmin: true }
  },
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

// 全局前置守卫
router.beforeEach(async (to, from, next) => {
  // 只有在需要管理员权限且尚未验证过的情况下才检查
  if (to.meta.requiresAdmin && !hasAdminPrivileges.value) {
    try {
      const isAdmin = await invoker('check_admin_privileges')
      console.log('isAdmin: ', isAdmin)
      hasAdminPrivileges.value = isAdmin
      
      if (!isAdmin) {
        // 如果不是管理员且不是已经在管理员页面，则重定向
        if (to.name !== 'AdminRequired') {
          next({ name: 'AdminRequired' })
          return
        }
      }
    } catch (error) {
      console.error('检查管理员权限失败:', error)
      if (to.name !== 'AdminRequired') {
        next({ name: 'AdminRequired' })
        return
      }
    }
  }
  
  // 如果已经有管理员权限，但用户试图访问管理员要求页面，重定向到首页
  if (to.name === 'AdminRequired' && hasAdminPrivileges.value) {
    next({ name: 'Home' })
    return
  }
  
  next()
})

// export default router 