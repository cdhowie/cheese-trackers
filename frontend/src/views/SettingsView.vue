<script setup>
import { ref } from 'vue';
import { load, save } from '@/settings.js';

const saved = ref(false);
const settings = ref(load());

function saveSettings() {
    save(settings.value);

    setTimeout(() => { saved.value = false; }, 1000);
    saved.value = true;
}
</script>

<template>
    <form class="container">
        <div class="row">
            <div class="col-12">
                <label for="discordUsernameEntry" class="form-label">Your Discord
                    username</label>
                <input id="discordUsernameEntry" class="form-control" type="text" placeholder="Discord username"
                    v-model="settings.discordUsername">
            </div>
        </div>
        <div class="row mt-2">
            <div class="col-12 text-center">
                <button class="btn" :class="{ 'btn-primary': !saved, 'btn-success': saved }" :disabled="saved"
                    @click.prevent="saveSettings">{{ saved ? 'Saved' : 'Save' }}</button>
            </div>
        </div>
    </form>
</template>
