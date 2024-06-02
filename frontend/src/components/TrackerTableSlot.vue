<script setup>
import { computed } from 'vue';

const slots = defineSlots();

const columns = computed(() => slots.activity ? 13 : 12);
</script>

<template>
    <tr v-if="$slots.banner">
        <td :colspan="columns" class="text-center">
            <slot name="banner"/>
        </td>
    </tr>
    <tr v-else>
        <td><slot name="name"/></td>
        <td><slot name="ping"/></td>
        <td><slot name="availability"/></td>
        <td><slot name="claim"/></td>
        <td><slot name="owner"/></td>
        <td><slot name="game"/></td>
        <td><slot name="progression"/></td>
        <td><slot name="completion"/></td>
        <td class="text-end"><slot name="checked"/></td>
        <td v-if="$slots.activity"><slot name="activity"/></td>
        <td class="text-start ps-0"><slot name="stillbk"/></td>
        <td><slot name="checks"/></td>
        <td><slot name="hints"/></td>
    </tr>
    <tr v-if="$slots.hintpane">
        <td :colspan="columns" class="container-fluid">
            <slot name="hintpane"/>
        </td>
    </tr>
</template>

<style scoped>
td {
    vertical-align: baseline;
}
</style>
