<script setup>
import { ref } from 'vue';
import * as settings from '@/settings.js';
import { pingPreference } from '@/types';

const saved = ref(false);

const editSettings = ref(settings.load());

function saveSettings() {
    settings.save(editSettings.value);

    setTimeout(() => { saved.value = false; }, 1000);
    saved.value = true;
}
</script>

<template>
    <form class="container" @submit.prevent="saveSettings">
        <div class="row">
            <div class="col-12 col-lg-6">
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
            <div v-if="!editSettings.auth?.token" class="col-12">
                <label for="discordUsernameEntry" class="form-label">Your Discord
                    username</label>
                <input id="discordUsernameEntry" class="form-control" type="text" placeholder="Discord username"
                    v-model="editSettings.unauthenticatedDiscordUsername">
            </div>
        </div>
        <div class="row mt-2">
            <div class="col-12 text-center">
                <input type="submit" class="btn" :class="{ 'btn-primary': !saved, 'btn-success': saved }" :disabled="saved"
                    :value="saved ? 'Saved' : 'Save'">
            </div>
        </div>
    </form>
</template>
