import { ref } from "vue";
import Joi from 'joi';

const SETTINGS_KEY = 'settings';

const SETTINGS_SCHEMA = Joi.object().keys({
    discordUsername: Joi.string().empty(""),
    defaultPingPreference: Joi.string().default("never"),
});

export const settings = ref(load());

function load() {
    try {
        return SETTINGS_SCHEMA.validate(
            JSON.parse(localStorage.getItem(SETTINGS_KEY))
        ).value || {};
    } catch (e) {
        return {};
    }
}

export function save(s) {
    const { value, error } = SETTINGS_SCHEMA.validate(s);
    if (error) {
        throw error;
    }

    localStorage.setItem(SETTINGS_KEY, JSON.stringify(value));
    settings.value = value;
}

window.addEventListener('storage', event => {
    if (event.key === null || event.key === SETTINGS_KEY) {
        settings.value = load();
    }
});
