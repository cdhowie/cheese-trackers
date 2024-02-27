<script setup>
import { computed } from 'vue';

const props = defineProps(['game']);

const manualGame = computed(() => {
    if (props.game) {
        const m = /^Manual_(.*)_.*$/.exec(props.game);

        return m?.[1];
    }
})
</script>

<template>
    <template v-if="manualGame">
        <i class="bi-person-fill-gear text-info" title="Manual game"></i> <span :title="props.game">{{ manualGame }}</span>
    </template>
    <template v-else>{{ props.game }}</template>
</template>
