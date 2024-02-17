<script setup>
import { computed, onBeforeUnmount, ref } from 'vue';
import { RouterLink, RouterView } from 'vue-router';
import { getSettings } from '@/api.js';
import { BUILD_VERSION } from './build';

const settings = ref({});

const newVersionAvailable = computed(() =>
  settings.value.build_version && settings.value.build_version !== BUILD_VERSION
);

async function updateSettings() {
  try {
    const { data } = await getSettings();
    settings.value = data;
  } catch (e) {
    console.log(`Failed to update settings: ${e}`);
  }
}

const interval = setInterval(updateSettings, 60 * 1000);

updateSettings();

onBeforeUnmount(() => {
  clearInterval(interval);
});
</script>

<template>
  <nav class="navbar navbar-expand bg-body-tertiary mb-3">
    <div class="container-fluid">
      <span class="navbar-brand">Cheese Trackers</span>
      <ul class="navbar-nav me-auto">
        <li class="nav-item">
          <RouterLink class="nav-link" active-class="active" to="/">Open tracker</RouterLink>
        </li>
        <li class="nav-item">
          <RouterLink class="nav-link" active-class="active" to="/settings">Settings</RouterLink>
        </li>
        <li class="nav-item">
          <RouterLink class="nav-link" active-class="active" to="/help">Help</RouterLink>
        </li>
      </ul>
    </div>
  </nav>

  <div v-if="settings.is_staging" class="alert alert-warning text-center">
    This is the staging server. The staging database will be periodically reset to facilitate testing. <b>Do not use this
      server as the authoritative tracker for your games.</b>
  </div>

  <RouterView />

  <footer class="mt-3 p-2 text-center bg-body-tertiary text-muted">
    <p>Built by The Incredible Wheel of Cheese for the Archipelago community. &#x1F9C0;</p>
    <p class="m-0">Inspired by radzprower's tracking spreadsheet.</p>
  </footer>

  <div class="toast-container position-fixed top-0 end-0 p-3">
    <div class="toast text-bg-info" :class="{ show: newVersionAvailable }">
      <div class="toast-body">
        A new build is available. Please refresh the page to load the new version.
      </div>
    </div>
  </div>
</template>

<style scoped></style>
