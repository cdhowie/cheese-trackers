<script setup>
import { computed } from 'vue';

const emit = defineEmits(['copy']);
const props = defineProps(['ping']);

const ICON_CLASSES_BY_PING = {
    yes: ['bi-bell-fill', 'text-success'],
    notes: ['bi-bell-fill', 'text-info'],
    no: ['bi-bell-slash-fill', 'text-danger'],
};

const iconClasses = computed(() => ICON_CLASSES_BY_PING[props.ping]);

const iconTooltip = computed(() =>
    props.ping === 'no' ?
        'Do not ping for this hint' :
        'Copy this hint with ping'
);

const role = computed(() => props.ping !== 'no' ? 'button' : '');

function clicked() {
    if (props.ping !== 'no') {
        emit('copy');
    }
}
</script>

<template>
    <i :class="iconClasses" :title="iconTooltip" @click="clicked()" :role="role"></i>
</template>
