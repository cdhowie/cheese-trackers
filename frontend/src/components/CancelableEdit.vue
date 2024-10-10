<script setup>
import { ref, watch } from 'vue';

const props = defineProps(['modelValue', 'reset']);
const emit = defineEmits(['update:modelValue']);

const editedValue = ref(props.modelValue);
const editingValue = ref(false);

watch(
    () => [props.modelValue, props.reset],
    () => {
        if (!editingValue.value) {
            editedValue.value = props.modelValue;
        }
    }
);

function save() {
    if (editingValue.value) {
        editingValue.value = false;
        emit('update:modelValue', editedValue.value);
    }
}

function cancel() {
    editedValue.value = props.modelValue;
    editingValue.value = false;
}

function edited(val) {
    editedValue.value = val;
    editingValue.value = true;
}
</script>

<template>
    <slot :value="editedValue" :save="save" :cancel="cancel" :edited="edited"/>
</template>
