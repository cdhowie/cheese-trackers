<script setup>
import { computed } from 'vue';
import { includes, filter, groupBy, mapValues, orderBy, sumBy } from 'lodash-es';
import { gameStatus } from '@/types';

const props = defineProps(['trackerData', 'summarizeBy']);

const summaryLabels = {
    discord_username: 'Player',
    game: 'Game',
}
const STATUSES = ['unblocked', 'bk', 'all_checks', 'done'];

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

function percent(num, den) {
    num ||= 0;
    den ||= 0;
    if (den === 0) {
        return '0%';
    }

    return `${(num / den) * 100}%`;
}

const sumKeys = computed(() => {
    return orderBy(Object.keys(summaryData.value));
});
</script>

<template>
    <table class="table table-border">
        <thead>
            <tr>
                <th class="text-end">{{ summaryLabels[summarizeBy] }}</th>
                <th>Games</th>
                <th>Checks</th>
            </tr>
        </thead>
        <tbody>
            <tr v-for="key in sumKeys">
                <td class="text-end">{{ key }}</td>
                <td>
                    <div class="progress">
                        <div v-for="status in STATUSES" class="progress-bar"
                            :class="[`bg-${gameStatus.byId[status].color}`]"
                            :style="{ width: percent(summaryData[key].byStatus[status]?.length, summaryData[key].count) }">
                        </div>
                    </div>
                </td>
                <td>
                    <div class="progress">
                        <div class="progress-bar bg-success"
                            :style="{ width: percent(summaryData[key].checksDone, summaryData[key].checksTotal) }">
                        </div>
                    </div>
                </td>
            </tr>
        </tbody>
    </table>
</template>
