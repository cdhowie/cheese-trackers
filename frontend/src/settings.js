import { ref } from "vue";
import Joi from 'joi';

const SETTINGS_KEY = 'settings';

const SETTINGS_SCHEMA = Joi.object().keys({
    defaultPingPreference: Joi.string().default("never"),
    unauthenticatedDiscordUsername: Joi.string().empty(""),

    auth: Joi.object().keys({
        token: Joi.string(),
        userId: Joi.number(),
        discordUsername: Joi.string(),
        discordSigninContinuationToken: Joi.string(),
        returnTo: Joi.string(),
    })
        .default({})
        .and('token', 'userId', 'discordUsername'),
}).prefs({ stripUnknown: true });

export const settings = ref(load());

function fallback() {
    return SETTINGS_SCHEMA.validate({}).value;
}

export function load() {
    try {
        return SETTINGS_SCHEMA.validate(
            JSON.parse(localStorage.getItem(SETTINGS_KEY))
        ).value || fallback();
    } catch (e) {
        return fallback();
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
