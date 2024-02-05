<script setup>
import { RouterLink, RouterView } from 'vue-router'
</script>

<template>
  <nav class="navbar navbar-expand bg-body-tertiary mb-3">
    <div class="container-fluid">
      <span class="navbar-brand">Async Multiworld Tracker</span>
      <ul class="navbar-nav me-auto">
        <li class="nav-item">
          <RouterLink class="nav-link" active-class="active" to="/">Open tracker</RouterLink>
        </li>
        <li class="nav-item">
          <RouterLink class="nav-link" active-class="active" to="/settings">Settings</RouterLink>
        </li>
      </ul>
    </div>
  </nav>

  <RouterView />

  <footer class="mt-3 p-2 text-center bg-body-tertiary text-muted">
    <p>Built by The Incredible Wheel of Cheese for the Archipelago community. &#x1F9C0;</p>
    <p class="m-0">Inspired by radzprower's tracking spreadsheet.</p>
  </footer>
</template>

<style scoped></style>
