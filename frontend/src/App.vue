<script setup>
import { ref } from 'vue';
import { RouterLink, RouterView } from 'vue-router';
import { getSettings } from '@/api.js';

const settings = ref({});

getSettings().then(({ data }) => { settings.value = data; })
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
</template>

<style scoped></style>
