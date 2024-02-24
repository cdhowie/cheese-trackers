<script setup>
import { computed } from 'vue';
import { includes, filter, groupBy, mapValues, orderBy, sumBy } from 'lodash-es';
import { gameStatus } from '@/types';
import { percent } from '@/util';
import ChecksBar from './ChecksBar.vue';
import UsernameDisplay from './UsernameDisplay.vue';

const props = defineProps(['trackerData', 'summarizeBy']);

const summaryTypes = {
    game: {
        label: 'Game',
        key: g => g.game,
        sortKey: key => key.toLowerCase(),
    },
    owner: {
        label: 'Player',
        key: g => JSON.stringify([g.discord_username, g.claimed_by_ct_user_id]),
        sortKey: key => (JSON.parse(key)[0] || '').toLowerCase(),
        keyDisplay: {
            component: UsernameDisplay,
            map: key => {
                const [discordUsername, id] = JSON.parse(key);
                return discordUsername && {
                    discordUsername,
                    id: id === null ? undefined : id,
                };
            },
            bindTo: 'user',
        },
    },
};

const summaryType = computed(() => summaryTypes[props.summarizeBy]);

const STATUSES = ['unknown', 'unblocked', 'bk', 'open', 'all_checks', 'goal', 'done'];

const summaryData = computed(() => {
    return mapValues(
        groupBy(
            filter(
                (props.trackerData || {}).games,
                g => includes(STATUSES, g.status)
            ),
            summaryType.value.key
        ),
        games => ({
            count: games.length,
            byStatus: groupBy(games, 'status'),
            checksDone: sumBy(games, 'checks_done'),
            checksTotal: sumBy(games, 'checks_total'),
        })
    );
});

const sumKeys = computed(() => {
    return orderBy(Object.keys(summaryData.value), summaryType.value.sortKey);
});
</script>

<template>
    <table class="table table-border">
        <thead>
            <tr>
                <th class="text-end">{{ summaryType.label }}</th>
                <th></th>
                <th class="text-center">Games</th>
                <th class="text-center">Checks</th>
            </tr>
        </thead>
        <tbody>
            <tr v-for="key in sumKeys">
                <td class="text-end shrink-column">
                    <component v-if="summaryType.keyDisplay" :is="summaryType.keyDisplay.component"
                        v-bind="{ [summaryType.keyDisplay.bindTo]: summaryType.keyDisplay.map(key) }">
                    </component>
                    <template v-else>{{ key }}</template>
                </td>
                <td class="text-end shrink-column">{{ summaryData[key].count }}</td>
                <td class="align-middle">
                    <div class="progress">
                        <div v-for="status in STATUSES" class="progress-bar"
                            :class="[`bg-${gameStatus.byId[status].color}`]"
                            :style="{ width: `${percent(summaryData[key].byStatus[status]?.length, summaryData[key].count)}%` }">
                        </div>
                    </div>
                </td>
                <td class="align-middle">
                    <ChecksBar :done="summaryData[key].checksDone" :total="summaryData[key].checksTotal"></ChecksBar>
                </td>
            </tr>
        </tbody>
    </table>
</template>

<style scoped>
.shrink-column {
    width: 1px;
    white-space: nowrap;
}
</style>