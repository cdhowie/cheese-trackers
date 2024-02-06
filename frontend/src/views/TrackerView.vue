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

const gameExpanded = ref({});

function setAllExpanded(v) {
    trackerData.value.games.forEach(game => {
        gameExpanded.value[game.id] = v;
    });
}

const allExpanded = computed(() => {
    return trackerData.value.games.every(g => gameExpanded.value[g.id]);
});

const hintsColors = [
    { max: 0, color: 'secondary' },
    { max: 5, color: 'info' },
    { max: 10, color: 'warning' },
    { color: 'danger' }
];

function hintsClass(game) {
    // If a game has notes, we consider the number of hints to be 1 at a minimum
    // so the button won't be gray.
    const unfound = Math.max(unfoundHints(game), game.notes !== '' ? 1 : 0);

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
    if (loading.value) {
        return;
    }

    loading.value = true;
    error.value = undefined;

    try {
        const { data } = await apiGetTracker(props.aptrackerid);
        data.games.forEach(game => {
            game.$newnotes = game.notes;
        });
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
    if (loading.value) {
        return;
    }

    loading.value = true;

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
            loading.value = false;
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

function updateNotes(game) {
    updateGame(game, g => { g.notes = g.$newnotes; });
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
                    <td colspan="11">
                        <div class="btn-group">
                            <template v-for="filter in ownerFilters">
                                <input type="radio" class="btn-check" name="filter-owner" :id="`filter-owner-${filter.id}`"
                                    v-model="ownerFilter" :value="filter">
                                <label class="btn btn-sm btn-outline-secondary" :for="`filter-owner-${filter.id}`">
                                    {{ filter.label }}
                                </label>
                            </template>
                        </div>
                        <button class="btn btn-sm btn-secondary ms-2" @click="setAllExpanded(!allExpanded)">
                            {{ allExpanded ? 'Collapse' : 'Expand' }} all
                        </button>
                        <button class="btn btn-sm btn-primary ms-2" @click="loadTracker()"
                            :disabled="loading">Refresh</button>
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
                    <th>Checks</th>
                    <th>Hints</th>
                </tr>
            </thead>
            <tbody>
                <template v-for="game in filteredGames">
                    <tr>
                        <td>{{ game.name }}</td>
                        <td>
                            <button v-if="game.discord_ping || game.discord_username?.length" class="btn btn-sm"
                                :class="{ 'btn-outline-danger': !game.discord_ping, 'btn-outline-success': game.discord_ping }"
                                :disabled="loading" @click="togglePing(game)">
                                {{ game.discord_ping ? 'Yes' : 'No' }}
                            </button>
                        </td>
                        <td>
                            <button v-if="settings.discordUsername && !game.discord_username?.length"
                                class="btn btn-sm btn-outline-secondary" :disabled="loading"
                                @click="claimGame(game)">Claim</button>

                            <template
                                v-if="settings.discordUsername && game.discord_username?.length && game.discord_username !== settings.discordUsername">
                                <button class="btn btn-sm btn-outline-secondary" :disabled="loading"
                                    data-bs-toggle="dropdown">Claim</button>
                                <div class="dropdown-menu text-warning p-3">
                                    <span class="text-warning me-2 d-inline-block align-middle">Another user has claimed
                                        this
                                        slot.</span>
                                    <button class="btn btn-sm btn-warning" @click="claimGame(game)">Claim anyway</button>
                                </div>
                            </template>

                            <button v-if="settings.discordUsername && game.discord_username === settings.discordUsername"
                                class="btn btn-sm btn-outline-warning" :disabled="loading"
                                @click="unclaimGame(game)">Release</button>
                        </td>
                        <td>
                            <span :class="{ 'text-muted': !game.discord_username?.length }">
                                {{ game.discord_username?.length ? game.discord_username : '(Unset)' }}
                            </span>
                        </td>
                        <td>{{ game.game }}</td>
                        <td>
                            <button class="btn btn-sm dropdown-toggle" :disabled="loading"
                                :class="[`btn-outline-${statusInfo[game.status].color}`]" data-bs-toggle="dropdown">
                                {{ statusInfo[game.status].name }}
                            </button>
                            <ul class="dropdown-menu">
                                <li v-for="status in statuses">
                                    <button class="dropdown-item" :class="[`text-${statusInfo[status].color}`]"
                                        :disabled="loading || status === game.status"
                                        @click="setGameStatus(game, status)">{{
                                            statusInfo[status].name }}</button>
                                </li>
                            </ul>
                        </td>
                        <td :class="[lastCheckedClass(game.last_checked)]">{{ game.last_checked ?
                            displayDateTime(game.last_checked) : 'Never' }}</td>
                        <td>
                            <button class="btn btn-sm btn-outline-secondary" :disabled="loading"
                                @click="updateLastChecked(game)">Update</button>
                        </td>
                        <td>{{ displayDateTime(game.last_activity) }}</td>
                        <td class="align-middle">
                            <div class="progress" style="position: relative;">
                                <div style="position: absolute; width: 100%; height: 100%; text-align: center">
                                    {{ game.checks_done }} / {{ game.checks_total }}
                                </div>
                                <div class="progress-bar overflow-visible"
                                    :class="{ 'bg-success': game.checks_done === game.checks_total }"
                                    :style="{ width: `${checksCompletePct(game)}%` }">
                                </div>
                            </div>
                        </td>
                        <td>
                            <button class="btn btn-sm" :class="[hintsClass(game)]"
                                @click="gameExpanded[game.id] = !gameExpanded[game.id]">
                                {{ unfoundHints(game) }}<template v-if="game.notes !== ''">*</template>
                            </button>
                        </td>
                    </tr>
                    <tr v-if="gameExpanded[game.id]">
                        <td colspan="11" class="container-fluid">
                            <div class="row">
                                <div class="col-6">
                                    <div class="fw-bold">Unfound hints</div>
                                    <div v-if="(hintsByFinder[game.id] || []).filter(h => !h.found).length === 0"
                                        class="text-muted">
                                        There are no unfound hints right now.
                                    </div>
                                    <div v-else class="row justify-content-center">
                                        <div class="col-auto">
                                            <table class="table table-responsive">
                                                <tr v-for="hint in hintsByFinder[game.id].filter(h => !h.found)">
                                                    <td class="text-end pe-0">
                                                        <span class="text-info bg-transparent p-0">{{
                                                            gameById[hint.receiver_game_id].name
                                                        }}</span>'s
                                                        <span class="text-info bg-transparent p-0">{{ hint.item }}</span>
                                                    </td>
                                                    <td class="ps-0 pe-0">&nbsp;is at&nbsp;</td>
                                                    <td class="text-start ps-0">
                                                        <span class="text-info bg-transparent p-0">{{ hint.location
                                                        }}</span>
                                                        <template v-if="hint.entrance !== 'Vanilla'"> ({{ hint.entrance
                                                        }})</template>
                                                    </td>
                                                </tr>
                                            </table>
                                        </div>
                                    </div>
                                </div>
                                <div class="col-6">
                                    <div class="fw-bold">Notes</div>
                                    <textarea placeholder="Enter any notes about your game here." class="form-control"
                                        rows="5" v-model="game.$newnotes" @blur="updateNotes(game)"
                                        @keyup.esc="game.$newnotes = game.notes"></textarea>
                                    <div class="text-muted">
                                        Saves automatically when you click off of the field. Press ESC to cancel any edits.
                                    </div>
                                </div>
                            </div>
                        </td>
                    </tr>
                </template>
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