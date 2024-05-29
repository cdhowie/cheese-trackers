<script setup>
const props = defineProps(['value']);
</script>

<template>
    <span v-if="props.value" :class="`text-${props.value.color}`">
        {{ props.value.label }}
    </span>
</template>