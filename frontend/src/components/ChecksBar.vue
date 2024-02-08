<script setup>
import { computed } from 'vue';

const props = defineProps(['done', 'total', 'showPercent']);

const percent = computed(() => {
    return ((props.done / props.total) * 100).toFixed(1);
});

</script>

<template>
    <div class="progress" style="position: relative;">
        <div style="position: absolute; width: 100%; height: 100%; text-align: center">
            {{ done }} / {{ total }}<template v-if="props.showPercent"> ({{ percent }}%)</template>
        </div>
        <div class="progress-bar overflow-visible" :class="{ 'bg-success': done === total }"
            :style="{ width: `${percent}%` }">
        </div>
    </div>
</template>
