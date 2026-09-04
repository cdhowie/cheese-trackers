<script setup>
import { ref, watch } from 'vue';
import moment from 'moment';

const model = defineModel();

const modelStr = ref('');

function modelUpdated() {
  if (model.value) {
    const m = moment(model.value);
    if (m.isValid()) {
      const d = m.toDate();
      d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
      modelStr.value = d.toISOString().slice(0, 16);
      return;
    }
  }

  modelStr.value = '';
}

modelUpdated();

function valueUpdated(value) {
  const m = moment(value);

  model.value = m.isValid() ? m : undefined;
}

watch(model, modelUpdated);
</script>

<template>
  <input type="datetime-local" :value="modelStr" @input="valueUpdated($event.target.value)">
</template>
