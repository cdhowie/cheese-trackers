import axios from 'axios';

import * as settings from './settings';

const api_http = axios.create({
    baseURL: import.meta.env.DEV ? 'http://127.0.0.1:3000/api/' : '/api/',
});

api_http.interceptors.request.use(config => {
    const token = settings.settings.value.auth?.token;
    if (token) {
        config.headers.authorization = `Bearer ${token}`;
    }
    return config;
});

api_http.interceptors.response.use(
    r => r,
    r => {
        if (r.status === 401) {
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

export async function updateGame(tracker_id, game) {
    return api_http.request({
        method: 'put',
        url: `tracker/${tracker_id}/game/${game.id}`,
        data: game,
    });
}

export async function updateTracker(tracker) {
    return api_http.request({
        method: 'put',
        url: `tracker/${tracker.tracker_id}`,
        data: tracker,
    });
}

export async function getSettings() {
    return api_http.get('settings');
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