<script setup>
import { computed } from 'vue';

import { pingPreference, progressionStatus, completionStatus, getClaimingUserForGame } from '@/types';
import GameDisplay from './GameDisplay.vue';
import EnumDisplay from './EnumDisplay.vue';
import UsernameDisplay from './UsernameDisplay.vue';

const props = defineProps(['game']);

const completion = computed(() => completionStatus.byId[props.game.completion_status]);

const ping = computed(() =>
    pingPreference.byId[
        completion.value?.complete ? 'never' : props.game.discord_ping
    ]
);

const progression = computed(() => progressionStatus.byId[props.game.progression_status]);

const owner = computed(() => getClaimingUserForGame(props.game));
</script>

<template>
    <div class="card text-center">
        <div class="card-body">
            <h5 class="card-title">
                {{ props.game.name }}
            </h5>
            <div class="card-text">
                <UsernameDisplay :user="owner"/>
                <span class="text-secondary"> is playing </span>
                <GameDisplay :game="props.game.game"/>
            </div>
            <div class="card-text">
                <EnumDisplay :value="completion"/>
                <template v-if="!completion?.complete">
                    <span class="text-secondary"> and </span>
                    <EnumDisplay :value="progression"/>
                </template>
            </div>
            <div v-if="owner" class="card-text">
                Ping <EnumDisplay :value="ping" :label="ping?.pingWhen"/>
            </div>
        </div>
    </div>
</template>
