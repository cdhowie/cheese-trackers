import { createRouter, createWebHistory } from 'vue-router';
import DashboardView from '@/views/DashboardView.vue';
import TrackerViewProxy from '@/views/TrackerViewProxy.vue';
import SettingsView from '@/views/SettingsView.vue';
import HelpView from '@/views/HelpView.vue';
import AuthComplete from '@/views/AuthComplete.vue';

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: DashboardView,
    },
    {
      path: '/settings',
      name: 'settings',
      component: SettingsView,
    },
    {
      path: '/tracker/:aptrackerid',
      name: 'tracker',
      component: TrackerViewProxy,
      props: true,
    },
    {
      path: '/help',
      name: 'help',
      component: HelpView,
    },

    {
      path: '/auth/complete',
      name: 'authcomplete',
      component: AuthComplete,
    },
  ]
});

export default router;
