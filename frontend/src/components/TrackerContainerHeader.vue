<template>
    <div class="row fw-bold bg-body" style="position: sticky; top: 0; z-index: 100">
        <div class="col-1 align-self-center"><slot name="name"/></div>
        <div class="col-1 align-self-center"><slot name="ping"/></div>
        <div class="col-1 align-self-center"><slot name="availability"/></div>
        <div class="col-2 align-self-center"><slot name="owner"/></div>
        <div class="col-2 align-self-center"><slot name="game"/></div>
        <div class="col-2 align-self-center"><slot name="status"/></div>
        <div class="col-1 align-self-center"><slot name="lastactivity"/></div>
        <div class="col-1 align-self-center"><slot name="checks"/></div>
        <div class="col-1 align-self-center"><slot name="hints"/></div>
    </div>
</template>
