<template>
    <div class="row p-1" v-if="$slots.banner">
        <div class="col-12 align-self-center"><slot name="banner"/></div>
    </div>
    <div class="row p-1" v-else>
        <div class="col-1 align-self-center"><slot name="name"/></div>
        <div class="col-1 align-self-center"><slot name="ping"/></div>
        <div class="col-1 align-self-center">
            <slot name="availability"/>
        </div>
        <div class="col-2 align-self-center">
            <slot name="claim"/> <slot name="owner"/>
        </div>
        <div class="col-2 align-self-center"><slot name="game"/></div>
        <div class="col-2 align-self-center">
            <slot name="progression"/> <slot name="completion"/>
        </div>
        <div class="col-1 align-self-center">
            <slot name="checked"/> <slot name="activity"/> <slot name="stillbk"/>
        </div>
        <div class="col-1 align-self-center"><slot name="checks"/></div>
        <div class="col-1 align-self-center"><slot name="hints"/></div>
    </div>
    <div class="row p-1" v-if="$slots.hintpane">
        <div class="container-fluid">
            <slot name="hintpane"/>
        </div>
    </div>
</template>
