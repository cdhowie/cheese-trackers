<script setup>
import { computed, ref } from 'vue';
import router from '../router';

const trackerLinkRegexp = /^https:\/\/archipelago\.gg\/tracker\/(.+)$/;

const trackerLink = ref('');

const trackerLinkIsValid = computed(() => {
    return trackerLinkRegexp.test(trackerLink.value);
});

function goToTracker() {
    const m = trackerLinkRegexp.exec(trackerLink.value);
    if (m) {
        router.push(`/tracker/${m[1]}`);
    }
}
</script>

<template>
    <div class="container">
        <label for="trackerLinkEntry" class="form-label">Archipelago tracker link</label>
        <div class="input-group">
            <input id="trackerLinkEntry" placeholder="https://archipelago.gg/tracker/..." type="text" class="form-control"
                v-model="trackerLink" @keyup.enter="goToTracker">
            <button class="btn btn-primary btn-primary" :disabled="!trackerLinkIsValid" @click="goToTracker">
                Open
            </button>
        </div>
    </div>
</template>
