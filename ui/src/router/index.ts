import { createRouter, createWebHistory } from 'vue-router'
import Layout from '../components/layout/index.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      component: Layout,
      children: [
        {
          path: '',
          name: 'Dashboard',
          component: () => import('../views/dashboard/index.vue'),
          meta: {
            title: '控制台',
            description: '用更清晰的总览面板查看本机外设、在线状态与控制能力分布。',
          },
        },
        {
          path: 'pending-tasks',
          name: 'PendingTasks',
          component: () => import('../views/pending-tasks/index.vue'),
          meta: {
            title: '待处理任务',
            description: '聚焦等待执行、等待确认与等待同步的控制任务，减少无效切换。',
          },
        },
        {
          path: 'task-history',
          name: 'TaskHistory',
          component: () => import('../views/task-history/index.vue'),
          meta: {
            title: '任务记录',
            description: '保留关键动作的时间线和回执信息，便于回溯每一次设备控制结果。',
          },
        },
        {
          path: 'connected-devices',
          name: 'ConnectedDevices',
          component: () => import('../views/connected-devices/index.vue'),
          meta: {
            title: '设备管理',
            description: '查看哪些移动端已受信、谁在线，以及谁正在占用控制会话。',
          },
        },
        {
          path: 'features',
          name: 'Features',
          component: () => import('../views/features/index.vue'),
          meta: {
            title: '功能中心',
            description: '将电源、音量与扩展能力集中在同一套语义化控制界面中。',
          },
        },
      ],
    },
  ],
})

export default router
