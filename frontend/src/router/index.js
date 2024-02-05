import { createRouter, createWebHistory } from 'vue-router';
import OpenTrackerView from '@/views/OpenTrackerView.vue';
import TrackerView from '@/views/TrackerView.vue';
import SettingsView from '@/views/SettingsView.vue';

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'opentracker',
      component: OpenTrackerView,
    },
    {
      path: '/settings',
      name: 'settings',
      component: SettingsView,
    },
    {
      path: '/tracker/:aptrackerid',
      name: 'tracker',
      component: TrackerView,
      props: true,
    }
  ]
});

export default router;
