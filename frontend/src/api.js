import axios from 'axios';
import { ref } from 'vue';

import * as settings from './settings';

const api_http = axios.create({
    baseURL: import.meta.env.DEV ? 'http://127.0.0.1:3000/api/' : '/api/',
});

export const uiSettings = ref({});

function updateUiSettings(response) {
    const settingsJson = response.headers?.['x-ct-settings'];

    try {
        uiSettings.value = JSON.parse(settingsJson);
    } catch (e) {
        console.log("Could not parse x-ct-settings", e);
    }
}

api_http.interceptors.request.use(config => {
    const token = settings.settings.value.auth?.token;
    if (token) {
        config.headers.authorization = `Bearer ${token}`;
    }
    return config;
});

api_http.interceptors.response.use(
    r => {
        updateUiSettings(r);
        return r;
    },
    r => {
        if (r.response) {
            updateUiSettings(r.response);
        } else if (r.headers) {
            updateUiSettings(r);
        }

        if ((r.status || r.response?.status) === 401) {
            const s = settings.load();
            s.auth = {};
            settings.save(s);
        }

        throw r;
    }
);

export async function getTracker(id) {
    return api_http.get(`tracker/${id}`);
}

export async function createTracker(url) {
    return api_http.post('tracker', { url });
}

export async function getDashboardTrackers() {
    return api_http.get('/dashboard/tracker');
}

export async function updateGame(tracker_id, game) {
    return api_http.request({
        method: 'put',
        url: `tracker/${tracker_id}/game/${game.id}`,
        data: game,
    });
}

export async function updateHint(tracker_id, hint) {
    return api_http.request({
        method: 'put',
        url: `tracker/${tracker_id}/hint/${hint.id}`,
        data: hint,
    });
}

export async function updateTracker(tracker) {
    return api_http.request({
        method: 'put',
        url: `tracker/${tracker.tracker_id}`,
        data: tracker,
    });
}

export async function setDashboardOverrideStatus(tracker_id, visibility) {
    return api_http.request({
        method: 'put',
        url: `tracker/${tracker_id}/dashboard_override`,
        data: { visibility }
    });
}

export async function ping() {
    return api_http.get('ping');
}

export async function authBegin() {
    return api_http.get('auth/begin');
}

export async function authComplete(data) {
    return api_http.request({
        method: 'post',
        url: 'auth/complete',
        data,
    });
}

export async function createJsError(data) {
    return api_http.request({
        method: 'post',
        url: 'jserror',
        data,
    });
}

export async function getApiKey() {
    return api_http.get('user/self/api_key');
}

export async function newApiKey() {
    return api_http.post('user/self/api_key');
}

export async function deleteApiKey() {
    return api_http.delete('user/self/api_key');
}
