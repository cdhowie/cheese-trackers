<script setup>
import { currentUser } from '@/settings';
import { usersEqual } from '@/types';

const props = defineProps(['user']);
</script>

<template>
    <template v-if="props.user?.discordUsername">
        <span :class="{ 'text-warning': props.user.isAway }">
            {{ props.user.discordUsername }}
        </span> <i
            v-if="props.user.isAway"
            title="Away"
            class="bg-transparent text-warning bi-hourglass-split"
        /> <i
            v-if="props.user.id === undefined"
            title="Unauthenticated"
            class="bg-transparent text-warning bi-unlock-fill"
        /> <i v-if="usersEqual(props.user, currentUser)"
            title="This is you"
            class="bg-transparent text-info bi-person-circle"/>
    </template>
    <span v-else class="text-muted">(Unclaimed)</span>
</template>
