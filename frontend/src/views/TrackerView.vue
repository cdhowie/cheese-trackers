<script setup>
import { computed, ref } from 'vue';
import { groupBy, keyBy, orderBy, sumBy, uniq } from 'lodash-es';
import { load as loadSettings } from '@/settings.js';
import { now } from '@/time.js';
import { getTracker as apiGetTracker, updateGame as apiUpdateGame } from '@/api.js';

const props = defineProps(['aptrackerid']);

const settings = loadSettings();

const loading = ref(false);
const error = ref(undefined);
const trackerData = ref(undefined);
const hintsByFinder = ref(undefined);
const gameById = ref(undefined);

const hintsColors = [
    { max: 5, color: 'info' },
    { max: 10, color: 'warning' },
    { color: 'danger' }
];

function hintsClass(game) {
    const unfound = unfoundHints(game);

    for (const c of hintsColors) {
        if (c.max === undefined || unfound <= c.max) {
            return `btn-outline-${c.color}`;
        }
    }
}

const ownerFilters = [
    { id: 'all', label: 'All', predicate: () => true },
    { id: 'mine', label: 'Mine', predicate: g => g.discord_username === settings.discordUsername },
    { id: 'unowned', label: 'Unowned', predicate: g => !g.discord_username?.length },
]
    .filter(f => f.id !== 'mine' || settings.discordUsername?.length);

const ownerFilter = ref(ownerFilters[0]);

const lastCheckedThresholds = [
    { hours: 48, color: 'danger' },
    { hours: 24, color: 'warning' },
    { color: 'success' }
];

function lastCheckedClass(checked) {
    if (!checked) {
        return 'text-danger';
    }

    const sinceMs = now.value - new Date(checked);
    const sinceHours = sinceMs / 1000 / 60 / 60;

    for (const t of lastCheckedThresholds) {
        if (t.hours === undefined || sinceHours >= t.hours) {
            return `text-${t.color}`;
        }
    }
}

function checksCompletePct(game) {
    return Math.floor((game.checks_done / game.checks_total) * 100);
}

const uniquePlayers = computed(() =>
    uniq(trackerData.value.games.map(g => g.discord_username).filter(i => i !== undefined)).length
);

const uniqueGames = computed(() =>
    uniq(trackerData.value.games.map(g => g.game)).length
);

const totalDoneChecks = computed(() =>
    sumBy(trackerData.value.games, 'checks_done')
);

const totalChecks = computed(() =>
    sumBy(trackerData.value.games, 'checks_total')
);

const statuses = [
    'unblocked',
    'bk',
    'all_checks',
    'done',
    'open',
    'released',
    'glitched',
];

const statusInfo = {
    unblocked: { name: 'Unblocked', color: 'light' },
    bk: { name: 'BK', color: 'danger' },
    all_checks: { name: 'All checks', color: 'warning' },
    done: { name: 'Done', color: 'success' },
    open: { name: 'Open', color: 'info' },
    released: { name: 'Released', color: 'secondary' },
    glitched: { name: 'Glitched', color: 'secondary' },
};

const filteredGames = computed(() =>
    orderBy(trackerData.value.games, g => g.name.toLowerCase()).filter(g =>
        ownerFilter.value.predicate(g)
    )
);

function displayDateTime(d) {
    if (d) {
        return new Date(d).toLocaleString();
    }
}

function unfoundHints(game) {
    return (hintsByFinder.value[game.id] || []).filter(
        hint => !hint.found
    ).length;
}

async function loadTracker() {
    loading.value = true;
    error.value = undefined;

    try {
        const { data } = await apiGetTracker(props.aptrackerid);
        trackerData.value = data;

        hintsByFinder.value = groupBy(trackerData.value.hints, 'finder_game_id');
        gameById.value = keyBy(trackerData.value.games, 'id');
    } catch (e) {
        error.value = e;
    } finally {
        loading.value = false;
    }
}

async function updateGame(game, mutator) {
    game.$is_updating = true;

    const data = { ...game };
    mutator(data);

    return apiUpdateGame(props.aptrackerid, data)
        .then(
            ({ status }) => status === 204,
            e => {
                console.error(`Failed to update game: ${e}`);
                return false;
            }
        )
        .then(saved => {
            if (saved) {
                mutator(game);
            }
            delete game.$is_updating;
        });
}

function claimGame(game) {
    updateGame(game, g => {
        g.discord_username = settings.discordUsername;
    });
}

function unclaimGame(game) {
    updateGame(game, g => {
        delete g.discord_username;
        g.discord_ping = false;
    });
}

function setGameStatus(game, status) {
    // HACK: We use setTimeout here because otherwise the dropdown becomes
    // disabled before Bootstrap closes the dropdown.  When it tries to do so,
    // it finds it disabled and won't close it.
    setTimeout(() => updateGame(game, g => { g.status = status; }));
}

function togglePing(game) {
    updateGame(game, g => { g.discord_ping = !g.discord_ping; });
}

function updateLastChecked(game) {
    const now = new Date();
    updateGame(game, g => { g.last_checked = now.toISOString(); });
}

loadTracker();
</script>

<template>
    <div v-if="loading && !trackerData" class="text-center">Loading tracker data...</div>
    <div v-if="error && !trackerData" class="text-center text-danger">Failed to load tracker data ({{ error.message }})
    </div>
    <template v-if="trackerData">
        <table class="table table-sm table-hover text-center">
            <thead style="position: sticky; top: 0; z-index: 100">
                <tr>
                    <td colspan="13">
                        <div class="btn-group">
                            <template v-for="filter in ownerFilters">
                                <input type="radio" class="btn-check" name="filter-owner" :id="`filter-owner-${filter.id}`"
                                    v-model="ownerFilter" :value="filter">
                                <label class="btn btn-sm btn-outline-secondary" :for="`filter-owner-${filter.id}`">
                                    {{ filter.label }}
                                </label>
                            </template>
                        </div>
                        <button class="btn btn-sm btn-secondary ms-2" @click="loadTracker()">Refresh</button>
                    </td>
                </tr>
                <tr>
                    <th>Name</th>
                    <th>Ping</th>
                    <th></th>
                    <th>Owner (Discord Username)</th>
                    <th>Game</th>
                    <th>Status</th>
                    <th colspan="2">Last Checked</th>
                    <th>Last Activity</th>
                    <th colspan="3">Checks</th>
                    <th>Hints</th>
                </tr>
            </thead>
            <tbody>
                <tr v-for="game in filteredGames">
                    <td>{{ game.name }}</td>
                    <td>
                        <button v-if="game.discord_ping || game.discord_username?.length" class="btn btn-sm"
                            :class="{ 'btn-outline-danger': !game.discord_ping, 'btn-outline-success': game.discord_ping }"
                            :disabled="loading || game.$is_updating" @click="togglePing(game)">
                            {{ game.discord_ping ? 'Yes' : 'No' }}
                        </button>
                    </td>
                    <td>
                        <button v-if="settings.discordUsername && !game.discord_username?.length"
                            class="btn btn-sm btn-outline-secondary" :disabled="loading || game.$is_updating"
                            @click="claimGame(game)">Claim</button>

                        <template
                            v-if="settings.discordUsername && game.discord_username?.length && game.discord_username !== settings.discordUsername">
                            <button class="btn btn-sm btn-outline-secondary" :disabled="loading || game.$is_updating"
                                data-bs-toggle="dropdown">Claim</button>
                            <div class="dropdown-menu text-warning p-3">
                                <span class="text-warning me-2 d-inline-block align-middle">Another user has claimed this
                                    slot.</span>
                                <button class="btn btn-sm btn-warning" @click="claimGame(game)">Claim anyway</button>
                            </div>
                        </template>

                        <button v-if="settings.discordUsername && game.discord_username === settings.discordUsername"
                            class="btn btn-sm btn-outline-warning" :disabled="loading || game.$is_updating"
                            @click="unclaimGame(game)">Release</button>
                    </td>
                    <td>
                        <span :class="{ 'text-muted': !game.discord_username?.length }">
                            {{ game.discord_username?.length ? game.discord_username : '(Unset)' }}
                        </span>
                    </td>
                    <td>{{ game.game }}</td>
                    <td>
                        <button class="btn btn-sm dropdown-toggle" :disabled="loading || game.$is_updating"
                            :class="[`btn-outline-${statusInfo[game.status].color}`]" data-bs-toggle="dropdown">
                            {{ statusInfo[game.status].name }}
                        </button>
                        <ul class="dropdown-menu">
                            <li v-for="status in statuses">
                                <button class="dropdown-item" :class="[`text-${statusInfo[status].color}`]"
                                    :disabled="loading || status === game.status" @click="setGameStatus(game, status)">{{
                                        statusInfo[status].name }}</button>
                            </li>
                        </ul>
                    </td>
                    <td :class="[lastCheckedClass(game.last_checked)]">{{ game.last_checked ?
                        displayDateTime(game.last_checked) : 'Never' }}</td>
                    <td>
                        <button class="btn btn-sm btn-outline-secondary" :disabled="loading || game.$is_updating"
                            @click="updateLastChecked(game)">Update</button>
                    </td>
                    <td>{{ displayDateTime(game.last_activity) }}</td>
                    <!--
                    <td class="text-end pe-0">{{ game.checks_done }}/</td>
                    <td class="text-start ps-0">{{ game.checks_total }}</td>
                    <td>{{ Math.floor((game.checks_done / game.checks_total) * 100) }}%</td>
                    -->
                    <td colspan="3" class="align-middle">
                        <div class="progress">
                            <div class="progress-bar overflow-visible"
                                :class="{ 'bg-success': game.checks_done === game.checks_total }"
                                :style="{ width: `${checksCompletePct(game)}%` }">
                            </div>
                        </div>
                    </td>
                    <td>
                        <button v-if="unfoundHints(game)" class="btn btn-sm" :class="[hintsClass(game)]"
                            data-bs-toggle="dropdown" data-bs-auto-close="outside">
                            {{ unfoundHints(game) }}
                        </button>
                        <div class="dropdown-menu p-3">
                            <ul class="m-0">
                                <template v-for="hint in hintsByFinder[game.id]">
                                    <li v-if="!hint.found">
                                        <span class="text-info">{{ gameById[hint.receiver_game_id].name }}</span>'s
                                        <span class="text-info">{{ hint.item }}</span> is at
                                        <span class="text-info">{{ hint.location }}</span>
                                        <template v-if="hint.entrance !== 'Vanilla'"> ({{ hint.entrance }})</template>
                                    </li>
                                </template>
                            </ul>
                        </div>
                    </td>
                </tr>
            </tbody>
        </table>
        <div class="container-fluid">
            <div class="row justify-content-center">
                <div class="col-12 col-lg-8 col-xl-6 col-xxl-5">
                    <table class="table table-sm text-center">
                        <thead>
                            <tr>
                                <th>Unique players</th>
                                <th>Unique games</th>
                                <th colspan="3">Total checks</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td>{{ uniquePlayers }}</td>
                                <td>{{ uniqueGames }}</td>
                                <td class="text-end pe-0">{{ totalDoneChecks }}/</td>
                                <td class="text-start ps-0">{{ totalChecks }}</td>
                                <td>{{ Math.floor((totalDoneChecks / totalChecks) * 100) }}%</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
        <div class="text-center">Last updated from Archipelago at {{ displayDateTime(trackerData.updated_at) }}
        </div>
    </template>
</template>