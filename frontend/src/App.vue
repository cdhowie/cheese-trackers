<script setup>
import { computed } from 'vue';
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { authBegin, ping, uiSettings as serverSettings } from '@/api.js';
import { BUILD_VERSION } from './build';
import * as settings from '@/settings';
import { filter, includes } from 'lodash-es';

const route = useRoute();

const localSettings = settings.settings;

const newVersionAvailable = computed(() =>
    serverSettings.value.build_version && serverSettings.value.build_version !== BUILD_VERSION
);

// Get the UI settings by making a no-op requset.
ping();

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

const banners = computed(() =>
    filter(
        serverSettings.value.banners,
        (banner) => !includes(localSettings.value.dismissedBanners, banner.id)
    )
);

function dismissBanner(id) {
    const s = settings.load();
    s.dismissedBanners.push(id);
    settings.save(s);
}
</script>

<template>
    <nav class="navbar navbar-expand-lg bg-body-tertiary mb-3">
        <div class="container-fluid">
            <span class="navbar-brand">Cheese Trackers</span>
            <button
                class="navbar-toggler"
                type="button"
                data-bs-toggle="collapse"
                data-bs-target="#navbar"
                aria-controls="navbar"
                aria-expanded="false"
                aria-label="Toggle navigation"
            >
                <span class="navbar-toggler-icon"></span>
            </button>
            <div class="collapse navbar-collapse" id="navbar">
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
                <hr>
                <template v-if="localSettings.auth?.discordUsername">
                    <span class="navbar-text">
                        Welcome, {{ localSettings.auth.discordUsername }}!
                    </span>
                    <button class="btn btn-sm btn-secondary ms-2" @click="logout">Log out</button>
                </template>
                <button v-else class="btn btn-sm btn-primary" @click="login">Sign in with Discord</button>
            </div>
        </div>
    </nav>

    <div
        v-for="banner in banners"
        class="alert text-center"
        :class="{
            [`alert-${banner.kind}`]: true,
            'alert-dismissible': banner.id,
        }"
    >
        <span v-html="banner.message"></span>
        <button v-if="banner.id" type="button" class="btn-close" @click="dismissBanner(banner.id)"></button>
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
