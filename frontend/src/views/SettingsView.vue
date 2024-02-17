<script setup>
import { ref } from 'vue';
import { settings, save } from '@/settings.js';
import { pingPreference } from '@/types';

const saved = ref(false);

// Make our own copy of the settings.  JSON stringify+parse is the simplest way
// to do this.
const editSettings = ref(JSON.parse(JSON.stringify(settings.value)));

function saveSettings() {
    save(editSettings.value);

    setTimeout(() => { saved.value = false; }, 1000);
    saved.value = true;
}
</script>

<template>
    <form class="container" @submit.prevent="saveSettings">
        <div class="row">
            <div class="col-12 col-lg-6">
                <label for="discordUsernameEntry" class="form-label">Your Discord
                    username</label>
                <input id="discordUsernameEntry" class="form-control" type="text" placeholder="Discord username"
                    v-model="editSettings.discordUsername">
            </div>
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
        </div>
        <div class="row mt-2">
            <div class="col-12 text-center">
                <input type="submit" class="btn" :class="{ 'btn-primary': !saved, 'btn-success': saved }" :disabled="saved"
                    :value="saved ? 'Saved' : 'Save'">
            </div>
        </div>
    </form>
</template>
