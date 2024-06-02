<script setup>
const props = defineProps(['showLastActivity']);
</script>

<template>
    <tr>
        <th><slot name="name"/></th>
        <th><slot name="ping"/></th>
        <th colspan="2"><slot name="availability"/></th>
        <th><slot name="owner"/></th>
        <th><slot name="game"/></th>
        <th colspan="2"><slot name="status"/></th>
        <th :colspan="props.showLastActivity ? 3 : 2">
            <slot name="lastactivity"/>
        </th>
        <th><slot name="checks"/></th>
        <th><slot name="hints"/></th>
    </tr>
</template>

<style scoped>
th {
    vertical-align: baseline;
}
</style>
