<script setup>
const props = defineProps(['done', 'total']);
</script>

<template>
    <div class="progress" style="position: relative;">
        <div style="position: absolute; width: 100%; height: 100%; text-align: center">
            {{ done }} / {{ total }}
        </div>
        <div class="progress-bar overflow-visible" :class="{ 'bg-success': done === total }"
            :style="{ width: `${(done / total) * 100}%` }">
        </div>
    </div>
</template>
