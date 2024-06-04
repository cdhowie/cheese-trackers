<script setup>
import { breakpoints } from '@/container-breakpoints.js';

const columns = ['name', 'ping', 'availability', 'owner', 'game', 'status', 'lastactivity', 'checks', 'hints'];
</script>

<template>
    <div
        class="row fw-bold bg-body border-bottom"
        style="position: sticky; top: 0; z-index: 100"
    >
        <div
            v-for="col in columns"
            :class="breakpoints[col]"
        >
            <slot :name="col"/>
        </div>
    </div>
</template>
