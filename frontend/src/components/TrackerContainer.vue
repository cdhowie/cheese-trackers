<!--
    Bootstrap container/row-based layout.  Use with TrackerContainerHeader and
    TrackerContainerSlot.
-->
<script setup>
import { DynamicScroller, DynamicScrollerItem } from 'vue-virtual-scroller';

const props = defineProps(['items']);
</script>

<template>
    <div class="container-fluid text-center">
        <slot name="head"/>
        <div v-if="!props.items?.length" class="row p-1">
            <div class="col-12 align-self-center text-center text-muted">
                No slots match the selected filters.
            </div>
        </div>
    </div>
    
    <DynamicScroller :items="props.items" :min-item-size="40" class="scroller">
        <template #default="{ item, index, active }">
            <DynamicScrollerItem
                :item="item"
                :active="active"
                :data-index="index"
                watch-data
            >
                <div class="container-fluid text-center">
                    <slot name="game" :game="item"/>
                </div>
            </DynamicScrollerItem>
        </template>
    </DynamicScroller>
</template>

<style scoped>
.scroller {
    height: 80vh;
}
</style>
