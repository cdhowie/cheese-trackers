<script setup>
/*
 * When a tracker is open and the user navigates to a different tracker in the
 * browser's history, the aptrackerid property gets changed but the component
 * doesn't get recreated since it's the same.  When this happens, there is quite
 * a lot of state that needs to be reset, and any inflight requests would also
 * need to have their responses ignored.
 *
 * To simplify handling this correctly, this proxy component is used instead.
 * When the tracker ID gets changed as the result of navigation, the key is also
 * changed, which causes Vue to recreate the TrackerView component from scratch.
 */
import { ref, watch } from 'vue';

import TrackerView from './TrackerView.vue';

const props = defineProps(['aptrackerid']);
const key = ref(0);

watch(
  () => props.aptrackerid,
  () => { key.value += 1; }
);
</script>

<template>
  <TrackerView :key="key" :aptrackerid="props.aptrackerid"/>
</template>
