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
            <div class="col-12">
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
