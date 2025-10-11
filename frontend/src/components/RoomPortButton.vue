<script setup>
import { copy as clipboardCopy } from '@/clipboard';
import { computed } from 'vue';

const props = defineProps(['host', 'port', 'stale']);

const roomHostAndPort = computed(() => {
  if (props.host && props.port) {
    return `${props.host}:${props.port}`;
  }
});
</script>

<template>
  <button
      type="button"
      v-if="roomHostAndPort"
      class="badge border border-0"
      :class="{
        'text-bg-info': !props.stale,
        'text-bg-warning': props.stale,
      }"
      @click="clipboardCopy(roomHostAndPort)"
  >
      <i class="bi-ethernet"></i> <span class="font-monospace" style="line-height: 0"
      >
          {{ roomHostAndPort }}
      </span>
  </button>
</template>
