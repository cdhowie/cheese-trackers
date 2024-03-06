<script setup>
import { computed } from 'vue';

const props = defineProps(['locked', 'loading', 'isOwner']);

const emit = defineEmits(['click']);

const buttonTitle = computed(() =>
    props.locked ? 'Allow anyone to change this setting.' :
    'Prevent changes to this setting by other users.'
);
</script>

<template>
    <button
        v-if="props.isOwner"
        type="button"
        :disabled="props.loading || !props.isOwner"
        :title="buttonTitle"
        class="btn"
        :class="`btn-outline-${props.locked ? 'success' : 'warning'}`"
        @click="emit('click')">
        <i :class="{
            'bi-unlock-fill': !props.locked,
            'bi-lock-fill': props.locked,
        }"></i>
    </button>
    <span 
        v-else-if="props.locked"
        class="input-group-text text-danger"
        title="This setting can only be changed by the organizer.">
        <i class="bi-lock-fill"></i>
    </span>
</template>
