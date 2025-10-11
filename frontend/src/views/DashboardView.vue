<script setup>
import { computed, ref, watch } from 'vue';
import { RouterLink } from 'vue-router';
import router from '../router';
import { settings } from '@/settings';
import { getDashboardTrackers, createTracker } from '@/api';
import moment from 'moment';
import { orderBy } from 'lodash-es';

import Repeat from '@/components/Repeat.vue';
import RoomPortButton from '@/components/RoomPortButton.vue';

const trackerLoading = ref(false);
const trackerError = ref(undefined);
const trackerLink = ref('');

async function goToTracker() {
    trackerLoading.value = true;
    trackerError.value = undefined;

    try {
        const { data } = await createTracker(trackerLink.value);
        router.push(`/tracker/${data.tracker_id}`);
    } catch (e) {
        trackerError.value = e;
    } finally {
        trackerLoading.value = false;
    }
}

function trackerHost(tracker) {
    if (tracker.room_link?.length) {
        try {
            return (new URL(tracker.room_link)).hostname;
        } catch (e) {}
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
            <input
                id="trackerLinkEntry"
                placeholder="https://archipelago.gg/tracker/..."
                type="text"
                class="form-control"
                v-model="trackerLink"
                :disabled="trackerLoading"
                @keyup.enter="goToTracker">
            <button
                class="btn btn-primary btn-primary"
                :disabled="trackerLoading || trackerLink === ''"
                @click="goToTracker"
            >
                <span v-if="trackerLoading" class="spinner-border spinner-border-sm"/>
                <template v-else>Open</template>
            </button>
        </div>
        <div v-if="trackerError" class="text-danger">
            Failed to load tracker ({{ trackerError.message }})
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
                    <th>Room</th>
                    <th>Last activity</th>
                </tr>
            </thead>
            <tbody>
                <tr v-for="tracker in myTrackers">
                    <td>
                        <RouterLink :to="`/tracker/${tracker.tracker_id}`">
                            {{ tracker.title || 'Untitled tracker' }}
                        </RouterLink> <i
                            v-if="tracker.dashboard_override_visibility"
                            class="bi-eye-fill"
                            title="Followed"
                        /> <template
                            v-if="tracker.owner_discord_username"
                        >
                            by {{ tracker.owner_discord_username }}
                        </template>
                    </td>
                    <td>
                        <a
                            v-if="tracker.room_link?.length"
                            :href="tracker.room_link"
                            target="_blank"
                            alt="Room"
                            class="badge text-bg-info"
                        >
                            <i class="bi-door-open-fill"></i>
                        </a> <RoomPortButton
                            :host="trackerHost(tracker)"
                            :port="tracker.last_port"
                            :stale="tracker.last_port_is_stale"
                        />
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
