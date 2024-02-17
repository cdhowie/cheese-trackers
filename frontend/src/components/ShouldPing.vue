<script setup>
const props = defineProps(['value']);

const classes = {
    yes: ['text-success', 'bi-check-circle-fill'],
    notes: ['text-info', 'bi-info-circle-fill'],
    no: ['text-danger', 'bi-x-circle-fill'],
};
</script>

<template>
    <i :class="classes[props.value]"></i>
</template>