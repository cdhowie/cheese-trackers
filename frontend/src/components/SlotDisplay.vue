<script setup>
import { ref } from 'vue';
import { useFloating, flip, autoUpdate, shift, offset } from '@floating-ui/vue';

import SlotSummary from './SlotSummary.vue';

const props = defineProps(['game']);

const show = ref(false);

const reference = ref(null);
const floating = ref(null);

const { floatingStyles } = useFloating(reference, floating, {
    placement: 'bottom',
    middleware: [
        offset(10),
        flip(),
        shift(),
    ],
    whileElementsMounted: autoUpdate,
});
</script>

<template>
    <span
        ref="reference"
        @mouseenter="show = true"
        @mouseleave="show = false"
    >{{ props.game.name }}</span>
    <Teleport to="body">
        <div ref="floating" class="popup" :style="floatingStyles" v-show="show">
            <SlotSummary :game="props.game"/>
        </div>
    </Teleport>
</template>

<style scoped>
.popup {
    z-index: 1000;
}
</style>