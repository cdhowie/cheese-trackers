<script setup>
import { range } from 'lodash-es';

const props = defineProps(['times']);
</script>

<template>
    <template v-for="i in range(props.times)">
        <slot></slot>
    </template>
</template>
