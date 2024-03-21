<script setup>
import { computed, onBeforeUnmount, ref } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { getSettings, authBegin } from '@/api.js';
import { BUILD_VERSION } from './build';
import * as settings from '@/settings';

const route = useRoute();

const localSettings = settings.settings;
const serverSettings = ref({});

const newVersionAvailable = computed(() =>
    serverSettings.value.build_version && serverSettings.value.build_version !== BUILD_VERSION
);

async function updateSettings() {
    try {
        const { data } = await getSettings();
        serverSettings.value = data;
    } catch (e) {
        console.log(`Failed to update settings: ${e}`);
    }
}

const interval = setInterval(updateSettings, 60 * 1000);

updateSettings();

onBeforeUnmount(() => {
    clearInterval(interval);
});

async function login() {
    const { data } = await authBegin();

    const s = settings.load();
    s.auth = {
        discordSigninContinuationToken: data.continuation_token,
        returnTo: route.path,
    };
    settings.save(s);

    document.location = data.auth_url;
}

function logout() {
    const s = settings.load();
    s.auth = {};
    settings.save(s);
}
</script>

<template>
    <nav class="navbar navbar-expand bg-body-tertiary mb-3">
        <div class="container-fluid">
            <span class="navbar-brand">Cheese Trackers</span>
            <ul class="navbar-nav me-auto">
                <li class="nav-item">
                    <RouterLink class="nav-link" active-class="active" to="/">Dashboard</RouterLink>
                </li>
                <li class="nav-item">
                    <RouterLink class="nav-link" active-class="active" to="/settings">Settings</RouterLink>
                </li>
                <li class="nav-item">
                    <RouterLink class="nav-link" active-class="active" to="/help">Help</RouterLink>
                </li>
            </ul>
            <template v-if="localSettings.auth?.discordUsername">
                Welcome, {{ localSettings.auth.discordUsername }}!
                <button class="btn btn-sm btn-secondary ms-2" @click="logout">Log out</button>
            </template>
            <button v-else class="btn btn-sm btn-primary" @click="login">Sign in with Discord</button>
        </div>
    </nav>

    <div v-if="serverSettings.is_staging" class="alert alert-warning text-center">
        This is the staging server. The staging database will be periodically
        reset to facilitate testing. <b>Do not use this server as the
            authoritative tracker for your games.</b>
    </div>

    <RouterView />

    <footer class="mt-3 p-2 text-center bg-body-tertiary text-muted">
        <p>Built by The Incredible Wheel of Cheese for the Archipelago community. &#x1F9C0;</p>
        <p class="m-0">Inspired by RadzPrower's tracking spreadsheet.</p>
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
