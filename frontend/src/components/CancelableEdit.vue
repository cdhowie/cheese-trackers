<script setup>
import { ref, watch } from 'vue';

const model = defineModel();

const editedValue = ref(model.value);
const editingValue = ref(false);

watch(
    () => model.value,
    () => {
        if (!editingValue.value) {
            editedValue.value = model.value;
        }
    }
);

function save() {
    if (editingValue.value) {
        editingValue.value = false;
        model.value = editedValue.value;
    }
}

function cancel() {
    editedValue.value = model.value;
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
