<script setup>
import { computed } from 'vue';

import { pingPreference, progressionStatus, completionStatus } from '@/types';
import GameDisplay from './GameDisplay.vue';
import EnumDisplay from './EnumDisplay.vue';

const props = defineProps(['game']);

const completion = computed(() => completionStatus.byId[props.game.completion_status]);

const ping = computed(() =>
    pingPreference.byId[
        completion.value?.complete ? 'never' : props.game.discord_ping
    ]
);

const progression = computed(() => progressionStatus.byId[props.game.progression_status]);
</script>

<template>
    <div class="card">
        <div class="card-body">
            <h5 class="card-title">
                {{ props.game.name }}
            </h5>
            <div class="card-text">
                Playing: <GameDisplay :game="props.game.game"/>
            </div>
            <div class="card-text">
                Status: <EnumDisplay :value="completion"/>
                <template v-if="!completion?.complete"> and <EnumDisplay :value="progression"/></template>
            </div>
            <div class="card-text">
                Ping: <EnumDisplay :value="ping"/>
            </div>
        </div>
    </div>
</template>
