<script setup>
import { computed, ref, watch } from 'vue';
import { RouterLink } from 'vue-router';
import router from '../router';
import { settings } from '@/settings';
import { getDashboardTrackers } from '@/api';
import moment from 'moment';
import { orderBy } from 'lodash-es';

import Repeat from '@/components/Repeat.vue';

const trackerLinkRegexp = /^https:\/\/archipelago\.gg\/tracker\/([^/]+)/;

const trackerLink = ref('');

const trackerLinkIsValid = computed(() => {
    return trackerLinkRegexp.test(trackerLink.value);
});

function goToTracker() {
    const m = trackerLinkRegexp.exec(trackerLink.value);
    if (m) {
        router.push(`/tracker/${m[1]}`);
    }
}

const myTrackers = ref(undefined);
const myTrackersError = ref(undefined);
const myTrackersLoading = ref(false);
let myTrackersLoadToken;

async function loadDashboard() {
    myTrackersError.value = undefined;
    myTrackersLoading.value = false;

    if (!settings.value.auth?.token) {
        myTrackers.value = undefined;
        return;
    }

    const token = {};
    myTrackersLoadToken = token;
    myTrackersLoading.value = true;

    try {
        const { data } = await getDashboardTrackers();

        if (myTrackersLoadToken !== token) {
            return;
        }

        myTrackers.value = orderBy(data, 'last_activity', 'desc');
    } catch (e) {
        if (myTrackersLoadToken !== token) {
            return;
        }

        myTrackersError.value = e;
    }

    myTrackersLoading.value = false;
}

loadDashboard();

watch(
    () => settings.value.auth?.token,
    () => loadDashboard()
);
</script>

<template>
    <div class="container">
        <h2>Find/create tracker</h2>
        <div class="input-group">
            <input id="trackerLinkEntry" placeholder="https://archipelago.gg/tracker/..." type="text" class="form-control"
                v-model="trackerLink" @keyup.enter="goToTracker">
            <button class="btn btn-primary btn-primary" :disabled="!trackerLinkIsValid" @click="goToTracker">
                Open
            </button>
        </div>
    </div>
    <div v-if="settings.auth?.token" class="container mt-3">
        <h2>My Trackers</h2>
        <table v-if="myTrackersLoading" class="table placeholder-wave">
            <thead>
                <tr>
                    <th><div class="placeholder w-100"></div></th>
                    <th><div class="placeholder w-100"></div></th>
                </tr>
            </thead>
            <tbody>
                <Repeat times="3">
                    <tr>
                        <td><div class="placeholder bg-secondary w-100"></div></td>
                        <td><div class="placeholder bg-secondary w-100"></div></td>
                    </tr>
                </Repeat>
            </tbody>
        </table>
        <table v-if="myTrackers" class="table">
            <thead>
                <tr>
                    <th>Tracker</th>
                    <th>Last activity</th>
                </tr>
            </thead>
            <tbody>
                <tr v-for="tracker in myTrackers">
                    <td>
                        <RouterLink :to="`/tracker/${tracker.tracker_id}`">
                            {{ tracker.title || 'Untitled tracker' }}
                        </RouterLink>
                        <template v-if="tracker.owner_discord_username">
                            by {{ tracker.owner_discord_username }}
                        </template>
                    </td>
                    <td>{{
                        tracker.last_activity ?
                        moment(tracker.last_activity).fromNow() :
                        'Never'
                    }}</td>
                </tr>
            </tbody>
        </table>
        <div v-if="myTrackersError" class="text-danger">
            Failed to load trackers ({{ myTrackersError.message }})
        </div>
    </div>
</template>
