<script setup>
import { breakpoints } from '@/container-breakpoints.js';

const props = defineProps(['isMine']);
</script>

<template>
    <div class="row p-1" v-if="$slots.banner">
        <div class="col-12 align-self-center"><slot name="banner"/></div>
    </div>
    <div class="row border-bottom" v-else>
        <div :class="breakpoints['name']"><slot name="name"/></div>
        <div :class="breakpoints['ping']"><slot name="ping"/></div>
        <div :class="breakpoints['availability']">
            <slot name="availability"/>
        </div>
        <div :class="breakpoints['owner']">
            <slot name="claim"/> <slot name="owner"/>
        </div>
        <div :class="breakpoints['game']"><slot name="game"/></div>
        <div :class="breakpoints['status']">
            <slot name="progression"/> <slot name="completion"/>
        </div>
        <div :class="breakpoints['lastactivity']">
            <slot name="checked"/> <slot name="activity"/> <slot name="stillbk"/>
        </div>
        <div :class="breakpoints['checks']"><slot name="checks"/></div>
        <div :class="breakpoints['hints']"><slot name="hints"/></div>
    </div>
    <div class="row p-1 border-bottom" v-if="$slots.hintpane">
        <div class="container-fluid">
            <slot name="hintpane"/>
        </div>
    </div>
</template>
