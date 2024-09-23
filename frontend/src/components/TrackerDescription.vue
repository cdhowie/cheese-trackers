<script setup>
import VueMarkdown from 'vue-markdown-render';
import mila from 'markdown-it-link-attributes';

const props = defineProps(['source']);

const plugins = [
    (i) => i.use(mila, {
        attrs: {
            target: '_blank',
        },
    }),
];
</script>

<template>
    <VueMarkdown
        :source="props.source"
        :options="{ linkify: true}"
        :plugins="plugins"
    />
</template>

<style>
p:last-child {
    margin-bottom: 0;
}
</style>
