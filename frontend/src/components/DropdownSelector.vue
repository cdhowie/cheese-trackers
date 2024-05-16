<script setup>
defineEmits(['selected']);
const props = defineProps(['options', 'value', 'disabled', 'icons']);
</script>

<template>
    <button class="btn btn-sm dropdown-toggle" :disabled="props.disabled" :class="[`btn-outline-${props.value.color}`]"
        data-bs-toggle="dropdown">
        <i v-if="props.icons" :class="`bi-${props.value.icon}`"></i>
        <template v-else>{{ props.value.label }}</template>
    </button>
    <ul class="dropdown-menu">
        <li v-for="option in props.options">
            <button class="dropdown-item" :class="{
                [`text-${option.color}`]: props.value !== option,
                active: props.value === option,
                [`bg-${option.color}`]: props.value === option,
                [`text-bg-${option.color}`]: props.value === option,
            }" :disabled="props.loading || props.value === option" @click="$emit('selected', option)"
            ><i v-if="option.icon" :class="`bi-${option.icon}`"></i> {{ option.label }}</button>
        </li>
    </ul>
</template>
