<!--
    Table-based tracker layout.  Use with TrackerLayoutHeader and
    TrackerLayoutSlot.
-->
<script setup>
const props = defineProps(['items']);
</script>

<template>
    <table class="table table-sm table-hover text-center">
        <thead style="position: sticky; top: 0; z-index: 100">
            <slot name="head"/>
        </thead>
        <tbody>
            <slot name="empty" v-if="!props.items?.length"/>
            <template v-for="item in props.items" :key="item.id">
                <slot name="game" :game="item"/>
            </template>
        </tbody>
    </table>
</template>
