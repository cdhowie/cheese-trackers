<script setup>
import { computed } from 'vue';

import { roundDown } from '@/util';

const props = defineProps(['done', 'total', 'showPercent']);

const percent = computed(() => {
    return roundDown((props.done / props.total) * 100, 1);
});

</script>

<template>
    <div class="progress mw-checks-bar" style="position: relative;">
        <div style="position: absolute; width: 100%; height: 100%; text-align: center">
            <template v-if="props.showPercent === 'only'">{{ percent }}%</template>
            <template v-else>{{ done }} / {{ total }}<template v-if="props.showPercent"> ({{ percent }}%)</template></template>
        </div>
        <div class="progress-bar overflow-visible" :class="{ 'bg-success': done === total }"
            :style="{ width: `${percent}%` }">
        </div>
    </div>
</template>

<style scoped>
.mw-checks-bar {
    min-width: 6.5em;
}
</style>
