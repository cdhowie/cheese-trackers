import axios from 'axios';

const api_http = axios.create({
    baseURL: import.meta.env.DEV ? 'http://127.0.0.1:3000/api/' : '/api/',
});

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

export async function getSettings() {
    return api_http.get('settings');
}
