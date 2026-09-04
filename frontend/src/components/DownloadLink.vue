<script setup>
import { makeFilenameSafe } from '@/util';
import { onMounted, useTemplateRef, watch } from 'vue';

const props = defineProps(['filename', 'content', 'contentType']);
const link = useTemplateRef('link');

function updateLink() {
  let blob;
  if (typeof props.content === 'string') {
    blob = new Blob([props.content], { type: props.contentType });
  } else if (props.content instanceof Blob) {
    blob = props.content;
  }

  if (!props.filename || !blob || !props.contentType) {
    link.value.href = "#";
    link.value.download = undefined;
    return;
  }

  const url = URL.createObjectURL(blob);

  link.value.href = url;
  link.value.download = makeFilenameSafe(props.filename);
}

watch(
  () => [props.filename, props.content, props.contentType],
  updateLink,
);

onMounted(updateLink);
</script>

<template>
  <a ref="link"><slot/></a>
</template>
