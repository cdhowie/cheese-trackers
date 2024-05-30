<script setup>
const props = defineProps(['value', 'label']);
</script>

<template>
    <span v-if="props.value" :class="`text-${props.value.color}`">
        {{ props.label || props.value.label }}
    </span>
</template>