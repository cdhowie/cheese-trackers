<script setup>
import { computed } from 'vue';
import { includes, filter, groupBy, mapValues, orderBy, sumBy } from 'lodash-es';
import { gameStatus } from '@/types';
import { percent } from '@/util';
import ChecksBar from './ChecksBar.vue';

const props = defineProps(['trackerData', 'summarizeBy']);

const summaryLabels = {
    discord_username: 'Player',
    game: 'Game',
}
const STATUSES = ['unblocked', 'bk', 'all_checks', 'done', 'open'];

const summaryData = computed(() => {
    return mapValues(
        groupBy(
            filter(
                (props.trackerData || {}).games,
                g => g[props.summarizeBy]?.length && includes(STATUSES, g.status)
            ),
            props.summarizeBy
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
    return orderBy(Object.keys(summaryData.value));
});
</script>

<template>
    <table class="table table-border">
        <thead>
            <tr>
                <th class="text-end">{{ summaryLabels[summarizeBy] }}</th>
                <th></th>
                <th class="text-center">Games</th>
                <th class="text-center">Checks</th>
            </tr>
        </thead>
        <tbody>
            <tr v-for="key in sumKeys">
                <td class="text-end shrink-column">{{ key }}</td>
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