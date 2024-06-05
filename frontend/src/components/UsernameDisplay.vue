<script setup>
import { isEqual } from 'lodash-es';
import { currentUser } from '@/settings';

const props = defineProps(['user']);
</script>

<template>
    <template v-if="props.user?.discordUsername">
        {{ props.user.discordUsername }} <i
            v-if="props.user.id === undefined"
            title="Unauthenticated"
            class="bg-transparent text-warning bi-unlock-fill"
        /> <i v-if="isEqual(props.user, currentUser)"
            title="This is you"
            class="bg-transparent text-info bi-person-circle"/>
    </template>
    <span v-else class="text-muted">(Unclaimed)</span>
</template>
