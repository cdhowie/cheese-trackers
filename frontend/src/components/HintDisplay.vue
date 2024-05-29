<script setup>
import { computed } from 'vue';

import { hintClassification, completionStatus } from '@/types';
import DropdownSelector from './DropdownSelector.vue';
import HintPingIcon from './HintPingIcon.vue';
import SlotDisplay from './SlotDisplay.vue';

defineEmits([
    'setClassification',
    'copy',
    'copyPing',
]);

const props = defineProps([
    'hint',
    'direction',
    'receiverGame',
    'finderGame',
    'disabled',
    'status',
    'showStatus',
]);

const HINT_STATUS_UI = {
    found: {
        iconclasses: ['bi-check-circle-fill', 'text-success'],
        icontooltip: 'Found',
        rowclasses: ['bg-success-subtle'],
    },
    notfound: {
        iconclasses: ['bi-x-circle-fill', 'text-danger'],
        icontooltip: 'Not found',
        rowclasses: ['bg-danger-subtle'],
    },
    useless: {
        iconclasses: ['bi-x-circle-fill', 'text-info'],
        icontooltip: 'Not found, receiving slot is done',
        rowclasses: ['bg-info-subtle'],
    },
}

const CAN_PING_BY_PREFERENCE = {
    liberally: 'yes',
    sparingly: 'yes',
    hints: 'yes',
    see_notes: 'notes',
    never: 'no',
};

const canPing = computed(() => {
    const otherSlot = props.direction === 'sent' ? props.finderGame : props.receiverGame;

    const otherSlotCompletion = completionStatus.byId[otherSlot.completion_status];

    if (otherSlotCompletion?.complete || !otherSlot.effective_discord_username) {
        return 'no';
    }

    return CAN_PING_BY_PREFERENCE[otherSlot.discord_ping];
});
</script>

<template>
    <tr class="bg-transparent">
        <td class="bg-transparent text-end pe-0">
            <template v-if="props.direction === 'received'">
                <span v-if="props.receiverGame" class="text-info">
                    <SlotDisplay :game="props.receiverGame"/>
                </span>
                <span v-else class="text-primary">(Item link)</span>'s
            </template>
            <span class="text-info p-0">{{ props.hint.item }}</span>
            <span class="ps-1">
                <DropdownSelector
                    :options="hintClassification"
                    :value="hintClassification.byId[props.hint.classification]"
                    :disabled="props.disabled"
                    :icons="true"
                    @selected="s => $emit('setClassification', s)"
                ></DropdownSelector>
            </span>
        </td>
        <td class="bg-transparent ps-0 pe-0">&nbsp;is&nbsp;at&nbsp;</td>
        <td class="bg-transparent text-start ps-0">
            <template v-if="props.direction === 'sent'" class="text-info">
                <span class="text-info">
                    <SlotDisplay :game="props.finderGame"/>
                </span>'s
            </template>
            <span class="text-info">{{ props.hint.location }}</span>
            <template v-if="props.hint.entrance !== 'Vanilla'"> ({{ props.hint.entrance }})</template
            > <i v-if="props.showStatus"
                :class="HINT_STATUS_UI[props.status].iconclasses"
                :title="HINT_STATUS_UI[props.status].icontooltip"></i
            > <HintPingIcon
                :ping="canPing"
                @copy="$emit('copyPing')"></HintPingIcon
            > <a
                href="#"
                class="text-light mw-copy-hint"
                @click.prevent="$emit('copy')"
                title="Copy to clipboard"
            >
                <i class="bi-copy"></i>
            </a>
        </td>
    </tr>
</template>

<style scoped>
.mw-copy-hint {
    visibility: hidden;
    text-decoration: none;
}

tr tr:hover .mw-copy-hint {
    visibility: visible;
}
</style>
