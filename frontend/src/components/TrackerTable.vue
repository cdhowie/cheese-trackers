<template>
    <table class="table table-sm table-hover text-center">
        <thead style="position: sticky; top: 0; z-index: 100">
            <slot name="head"/>
        </thead>
        <tbody>
            <slot name="body"/>
        </tbody>
    </table>
</template>
