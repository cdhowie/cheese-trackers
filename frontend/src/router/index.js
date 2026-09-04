import { createRouter, createWebHistory } from 'vue-router';
import DashboardView from '@/views/DashboardView.vue';
import TrackerViewProxy from '@/views/TrackerViewProxy.vue';
import SettingsView from '@/views/SettingsView.vue';
import HelpView from '@/views/HelpView.vue';
import AuthComplete from '@/views/AuthComplete.vue';
import CollectionRoomsView from '@/views/CollectionRoomsView.vue';
import CollectionRoomView from '@/views/CollectionRoomView.vue';

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: DashboardView,
    },
    {
      path: '/collection_room',
      name: 'collectionrooms',
      component: CollectionRoomsView,
    },
    {
      path: '/collection_room/:roomid',
      name: 'collectionroom',
      component: CollectionRoomView,
      props: true,
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
