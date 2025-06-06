<script setup>
import { ref, watch } from 'vue';
import * as settings from '@/settings.js';
import { pingPreference, sortModes } from '@/types';
import { getApiKey, newApiKey, deleteApiKey, getUserServerSettings, updateUserServerSettings } from '@/api';
import { copy as clipboardCopy } from '@/clipboard';

const saved = ref(false);

const editSettings = ref(settings.load());

function saveSettings() {
    settings.save(editSettings.value);

    setTimeout(() => { saved.value = false; }, 1000);
    saved.value = true;
}

const apiKey = ref(undefined);
const apiKeyLoading = ref(false);
const apiKeyError = ref(undefined);

const currentUser = settings.currentUser;

async function loadApiKey(by) {
    apiKeyLoading.value = true;
    apiKeyError.value = undefined;

    try {
        apiKey.value = (await by()).data;
    } catch (e) {
        apiKey.value = undefined;

        if (e.response?.status !== 401) {
            apiKeyError.value = e;
        }
    } finally {
        apiKeyLoading.value = false;
    }
}

function generateApiKey() {
    loadApiKey(newApiKey);
}

function clearApiKey() {
    loadApiKey(async () => {
        await deleteApiKey();
        return { data: undefined };
    })
}

const serverSettings = ref({});
const serverSettingsLoading = ref(false);
const serverSettingsError = ref(undefined);

async function serverSettingsRequest(req) {
    serverSettingsLoading.value = true;
    serverSettingsError.value = undefined;

    try {
        serverSettings.value = (await req()).data;
    } catch (e) {
        serverSettings.value = {};

        if (e.response?.status !== 401) {
            serverSettingsError.value = e;
        }
    } finally {
        serverSettingsLoading.value = false;
    }
}

async function loadServerSettings() {
    serverSettingsRequest(getUserServerSettings);
}

async function updateUserSettings(data) {
    const newData = { ...serverSettings.value, ...data };

    serverSettingsRequest(async () => updateUserServerSettings(newData));
}

function maybeLoadUserSettings(user) {
    if (user?.id !== undefined) {
        loadApiKey(getApiKey);
        loadServerSettings();
    }
}

watch(settings.currentUser, maybeLoadUserSettings);

maybeLoadUserSettings(settings.currentUser.value);
</script>

<template>
    <div class="container">
        <form @submit.prevent="saveSettings">
            <div class="row">
                <div v-if="!editSettings.auth?.token" class="col-12 col-lg-6">
                    <label for="discordUsernameEntry" class="form-label">Your Discord
                        username</label>
                    <input id="discordUsernameEntry" class="form-control" type="text" placeholder="Discord username"
                        v-model="editSettings.unauthenticatedDiscordUsername">
                </div>
                <div class="col-12" :class="{ 'col-lg-6': !editSettings.auth?.token }">
                    <label class="form-label">Default ping preference</label>
                    <div class="btn-group form-control border-0 p-0">
                        <template v-for="pref of pingPreference">
                            <input type="radio" class="btn-check" name="pingPref" :id="`ping-pref-${pref.id}`"
                                v-model="editSettings.defaultPingPreference" :value="pref.id">
                            <label class="btn" :class="[`btn-outline-${pref.color}`]" :for="`ping-pref-${pref.id}`">{{
                                pref.label }}</label>
                        </template>
                    </div>
                </div>
                <div class="col-12 col-lg-6">
                    <label class="form-label">Status selectors</label>
                    <div class="btn-group form-control border-0 p-0">
                        <template v-for="v in [false, true]">
                            <input type="radio" class="btn-check" name="statusIcons" :id="`status-icons-${v}`"
                                v-model="editSettings.statusIcons" :value="v">
                            <label class="btn btn-outline-secondary" :for="`status-icons-${v}`">
                                {{ v ? 'Icons' : 'Text' }}
                            </label>
                        </template>
                    </div>
                </div>
                <div class="col-12 col-lg-6">
                    <label class="form-label">Sort mode</label>
                    <div class="btn-group form-control border-0 p-0">
                        <template v-for="mode in sortModes">
                            <input type="radio" class="btn-check" name="sortMode" :id="`sort-mode-${mode.id}`"
                                v-model="editSettings.sortMode" :value="mode.id">
                            <label class="btn btn-outline-secondary" :for="`sort-mode-${mode.id}`">
                                {{ mode.label }}
                            </label>
                        </template>
                    </div>
                </div>
                <div class="col-12 col-lg-6">
                    <label class="form-label">Protect slots I haven't claimed</label>
                    <div class="btn-group form-control border-0 p-0">
                        <template v-for="v in [false, true]">
                            <input type="radio" class="btn-check" name="protectOtherSlots" :id="`protect-other-slots-${v}`"
                                v-model="editSettings.protectOtherSlots" :value="v">
                            <label class="btn btn-outline-secondary" :for="`protect-other-slots-${v}`">
                                {{ v ? 'Yes' : 'No' }}
                            </label>
                        </template>
                    </div>
                </div>
            </div>
            <div class="row mt-2">
                <div class="col-12 text-center">
                    <input type="submit" class="btn" :class="{ 'btn-primary': !saved, 'btn-success': saved }" :disabled="saved"
                        :value="saved ? 'Saved' : 'Save'">
                </div>
            </div>
        </form>

        <template v-if="currentUser?.id">
            <h2>Away</h2>

            <p>
                If you will be unable to play for an extended period, you can
                set yourself away.  This will annotate your slots so that others
                know about your absence.
            </p>

            <div v-if="serverSettingsLoading" class="text-center"><span class="spinner-border"/></div>
            <div v-else-if="serverSettingsError" class="text-center text-danger">Could not load away status: {{ serverSettingsError }}</div>
            <div v-else class="text-center">
                <div class="btn-group">
                    <button
                        class="btn"
                        :class="{
                            'btn-success': !serverSettings.is_away,
                            'btn-outline-success': serverSettings.is_away,
                        }"
                        @click.prevent="updateUserSettings({ is_away: false })"
                    >Not away</button>
                    <button
                        class="btn"
                        :class="{
                            'btn-warning': serverSettings.is_away,
                            'btn-outline-warning': !serverSettings.is_away,
                        }"
                        @click.prevent="updateUserSettings({ is_away: true })"
                    >Away</button>
                </div>
            </div>

            <h2>API key</h2>

            <p>
                An API key allows making requests to the Cheese Trackers API
                server on your behalf.  You can use this to develop your own
                tools, or you can supply it to a third-party tool to allow that
                tool access to your trackers.
            </p>

            <p class="text-warning">
                Giving this key to others will allow them access to read and
                modify data on Cheese Trackers with your identity.  Only give
                your API key to another tool if you trust the author not to
                abuse this access.
            </p>

            <p>
                Note that regenerating your API key will invalidate your
                existing API key.
            </p>

            <div v-if="apiKeyLoading" class="text-center"><span class="spinner-border"/></div>
            <div v-else-if="apiKeyError" class="text-center text-danger">Could not load API key: {{ apiKeyError }}</div>
            <div v-else class="row justify-content-center">
                <div class="col-xl-6 col-lg-8 col-12">
                    <label class="form-label">API key</label>
                    <div class="input-group">
                        <input type="text" readonly class="form-control" :value="apiKey" placeholder="You do not have an API key.">
                        <button class="btn btn-outline-secondary" title="Copy" @click.prevent="clipboardCopy(apiKey)" v-if="apiKey"><i class="bi-copy"/></button>
                        <button class="btn btn-outline-warning" title="Generate" @click.prevent="generateApiKey"><i class="bi-arrow-clockwise"/></button>
                        <button class="btn btn-outline-danger" title="Delete" @click.prevent="clearApiKey" v-if="apiKey"><i class="bi-trash-fill"/></button>
                    </div>
                </div>
            </div>
        </template>
    </div>
</template>
