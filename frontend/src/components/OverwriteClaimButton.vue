<script setup>
import { autoUpdate, flip, offset, shift, useFloating } from '@floating-ui/vue';
import { ref, useTemplateRef } from 'vue';

defineEmits(['claimed']);

const props = defineProps(['disabled']);

const dropdownShown = ref(false);

const { floatingStyles } = useFloating(
    useTemplateRef('reference'),
    useTemplateRef('floating'),
    {
        placement: 'bottom-start',
        middleware: [
            offset(2),
            flip(),
            shift(),
        ],
        whileElementsMounted: autoUpdate,
    },
);
</script>

<template>
    <button
        class="btn btn-sm btn-outline-secondary"
        :disabled="props.disabled"
        data-bs-toggle="dropdown"
        @[`shown.bs.dropdown`]="dropdownShown = true"
        @[`hidden.bs.dropdown`]="dropdownShown = false"
        ref="reference"
    >
        Claim
    </button>
    <div class="dropdown-menu" style="display: none"/>
    <Teleport to="body">
        <div
            v-if="dropdownShown"
            class="dropdown-menu show text-warning p-3"
            :style="floatingStyles"
            ref="floating"
        >
            <span class="text-warning me-2 d-inline-block align-middle">
                Another user has claimed this slot.
            </span>
            <button
                class="btn btn-sm btn-warning"
                :disabled="props.disabled"
                @click="$emit('claimed')"
            >
                Claim anyway
            </button>
        </div>
    </Teleport>
</template>
