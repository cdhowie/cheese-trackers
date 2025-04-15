<script setup>
// Wraps the logic of dropdown selector buttons.
//
// We have to teleport the dropdown menu to the body because the recycling
// container is incompatible with the dropdown menus; they always appear behind
// other rows.  To make this simpler and still get some of the benefits of
// Bootstrap's event handling, we listen to the Bootstrap-specific events
// indicating when the menu should be shown and manage the visibility and
// position ourselves.  We provide a dummy (display:none) dropdown-menu
// otherwise Bootstrap's code will throw an exception when attempting to show
// the menu.

import { autoUpdate, flip, offset, shift, useFloating } from '@floating-ui/vue';
import { ref } from 'vue';

defineEmits(['selected']);
const props = defineProps(['options', 'value', 'disabled', 'icons', 'readonly']);

const dropdownShown = ref(false);

const reference = ref(null);
const floating = ref(null);

const { floatingStyles } = useFloating(reference, floating, {
    placement: 'bottom-start',
    middleware: [
        offset(2),
        flip(),
        shift(),
    ],
    whileElementsMounted: autoUpdate,
});
</script>

<template>
    <span v-if="props.readonly" :class="`text-${props.value.color}`">
        <i v-if="props.icons" :title="props.value.label" :class="`bi-${props.value.icon}`"></i>
        <template v-else>{{ props.value.label }}</template>
    </span>
    <template v-else>
        <button
            class="btn btn-sm dropdown-toggle"
            :disabled="props.disabled"
            :class="[`btn-outline-${props.value.color}`]"
            data-bs-toggle="dropdown"
            @[`shown.bs.dropdown`]="dropdownShown = true"
            @[`hidden.bs.dropdown`]="dropdownShown = false"
            ref="reference"
        >
            <i v-if="props.icons" :class="`bi-${props.value.icon}`"></i>
            <template v-else>{{ props.value.label }}</template>
        </button>
        <ul class="dropdown-menu" style="display: none"/>
        <Teleport to="body">
            <ul
                v-if="dropdownShown"
                class="dropdown-menu show"
                :style="floatingStyles"
                ref="floating"
            >
                <li v-for="option in props.options">
                    <button
                        class="dropdown-item"
                        :class="{
                            [`text-${option.color}`]: props.value !== option,
                            active: props.value === option,
                            [`bg-${option.color}`]: props.value === option,
                            [`text-bg-${option.color}`]: props.value === option,
                        }"
                        :disabled="props.disabled || props.value === option"
                        @click="$emit('selected', option)"
                    ><i v-if="option.icon" :class="`bi-${option.icon}`"></i> {{ option.label }}</button>
                </li>
            </ul>
        </Teleport>
    </template>
</template>
