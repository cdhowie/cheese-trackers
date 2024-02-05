const SETTINGS_KEY = 'settings';

export function load() {
    try {
        return JSON.parse(localStorage.getItem(SETTINGS_KEY)) || {};
    } catch (e) {
        return {};
    }
}

export function save(s) {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
}
