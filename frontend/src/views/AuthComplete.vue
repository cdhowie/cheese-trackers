<script setup>
import { authComplete } from '@/api';
import { onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import router from '@/router';
import * as settings from '@/settings';

const route = useRoute();

const error = ref(undefined);

onMounted(async () => {
    const ls = settings.load();

    const { code, state } = route.query;
    const continuation_token = ls.auth?.discordSigninContinuationToken;

    if (!continuation_token) {
        if (ls.auth?.token) {
            // The user probably got here by clicking back.
            router.push('/');
        } else {
            error.value = "There is no outstanding authentication attempt.";
        }

        return;
    }

    try {
        const { data } = await authComplete({ code, state, continuation_token });

        const returnTo = ls.auth?.returnTo;

        const s = settings.load();
        s.auth = {
            token: data.token,
            userId: data.user_id,
            discordUsername: data.discord_username,
        };
        settings.save(s);

        router.push(returnTo || '/');
    } catch (e) {
        error.value = `${e}`;
    }
});
</script>

<template>
    <div class="text-center">
        <span v-if="error" class="text-danger">Sign-in failed: {{ error }}</span>
        <template v-else>Completing sign-in...</template>
    </div>
</template>
