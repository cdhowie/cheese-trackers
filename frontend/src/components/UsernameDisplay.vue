<script setup>
const props = defineProps(['user']);
</script>

<template>
    <template v-if="props.user?.discordUsername">
        {{ props.user.discordUsername }}
        <i v-if="props.user.id === undefined" title="Unauthenticated"
            class="bg-transparent text-warning bi-unlock-fill"></i>
    </template>
    <span v-else class="text-muted">(Unclaimed)</span>
</template>
