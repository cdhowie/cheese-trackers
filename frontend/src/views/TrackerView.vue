<script setup>
import { computed, ref } from 'vue';
import { groupBy, keyBy, orderBy, sumBy, uniq, mapValues, map, filter, reduce, join } from 'lodash-es';
import moment from 'moment';
import { load as loadSettings } from '@/settings';
import { now } from '@/time';
import { getTracker as apiGetTracker, updateGame as apiUpdateGame } from '@/api';
import { gameStatus } from '@/types';
import TrackerSummary from '@/components/TrackerSummary.vue';
import ChecksBar from '@/components/ChecksBar.vue';

const props = defineProps(['aptrackerid']);

const settings = loadSettings();

const loading = ref(false);
const error = ref(undefined);
const trackerData = ref(undefined);
const hintsByFinder = ref(undefined);
const hintsByReceiver = ref(undefined);
const gameById = ref(undefined);

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

    const prefix = gameExpanded.value[game.id] ? 'btn-' : 'btn-outline-';

    for (const c of hintsColors) {
        if (c.max === undefined || unfound <= c.max) {
            return `${prefix}${c.color}`;
        }
    }
}

const players = computed(() =>
    orderBy(
        uniq(
            filter(
                map(trackerData.value.games, 'discord_username'),
                i => i?.length
            )
        )
    )
);

const playersExceptSelf = computed(() =>
    filter(players.value, p => p !== settings.discordUsername)
);

const PLAYER_FILTER_ALL = Symbol();
const PLAYER_FILTER_UNOWNED = Symbol();

const playerFilter = ref(PLAYER_FILTER_ALL);

const lastCheckedThresholds = [
    { days: 2, color: 'danger' },
    { days: 1, color: 'warning' },
    { color: 'success' }
];

function gameLastUpdated(game) {
    return reduce(
        map(
            filter([game.last_checked, game.last_activity]),
            d => moment.utc(d)
        ),
        (a, b) => moment.max(a, b)
    );
}

function lastCheckedClass(game) {
    if (game.status === 'done') {
        return 'text-success';
    }

    const lastUpdated = gameLastUpdated(game);

    if (!lastUpdated) {
        return 'text-danger';
    }

    const sinceMs = moment.utc(now.value).diff(lastUpdated);
    const sinceDays = moment.duration(sinceMs).asDays();

    for (const t of lastCheckedThresholds) {
        if (t.days === undefined || sinceDays >= t.days) {
            return `text-${t.color}`;
        }
    }
}

function displayLastChecked(game) {
    if (game.status === 'done') {
        return '';
    }

    const lastUpdated = gameLastUpdated(game);

    if (!lastUpdated) {
        return 'Never';
    }

    const diff = moment.duration(
        Math.max(0, moment.utc(now.value).diff(lastUpdated))
    );
    return diff.asDays().toFixed(1);
}

const uniqueGames = computed(() =>
    uniq(trackerData.value.games.map(g => g.game)).length
);

const totalDoneChecks = computed(() =>
    sumBy(trackerData.value.games, 'checks_done')
);

const totalChecks = computed(() =>
    sumBy(trackerData.value.games, 'checks_total')
);

const statuses = gameStatus.map(i => i.id);
const statusFilter = ref(mapValues(gameStatus.byId, () => true));

const filteredGames = computed(() =>
    orderBy(trackerData.value.games, g => g.name.toLowerCase()).filter(g =>
        (
            playerFilter.value === PLAYER_FILTER_ALL ? true :
                playerFilter.value === PLAYER_FILTER_UNOWNED ? !g.discord_username?.length :
                    playerFilter.value === g.discord_username
        ) &&
        statusFilter.value[g.status]
    )
);

const gameExpanded = ref({});

function setAllExpanded(v) {
    trackerData.value.games.forEach(game => {
        gameExpanded.value[game.id] = v;
    });
}

const allExpanded = computed(() =>
    filteredGames.value.every(g => gameExpanded.value[g.id])
);

function displayDateTime(d) {
    if (moment.isMoment(d)) {
        return d.toDate().toLocaleString();
    }

    if (d) {
        return new Date(d).toLocaleString();
    }
}

const sentHints = ref(false);

const hintsByGame = computed(() => {
    return sentHints.value ? hintsByReceiver.value : hintsByFinder.value;
})

function unfoundHintsByGame(id) {
    return (hintsByGame.value?.[id] || []).filter(h => !h.found);
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
        hintsByReceiver.value = groupBy(trackerData.value.hints, 'receiver_game_id');
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
    if (game.notes !== game.$newnotes) {
        updateGame(game, g => { g.notes = g.$newnotes; });
    }
}

const showCopiedToast = ref(false);

function hintToString(hint) {
    const receiver = gameById.value[hint.receiver_game_id].name;
    const finder = gameById.value[hint.finder_game_id].name;
    const entrance = hint.entrance === 'Vanilla' ? '' : ` (${hint.entrance})`;
    return `${receiver}'s ${hint.item} is at ${finder}'s ${hint.location}${entrance}`;
}

function copyHints(hints) {
    clipboardCopy(join(map(hints, hintToString), '\n'))
}

function clipboardCopy(text) {
    navigator.clipboard.writeText(text);

    showCopiedToast.value = true;
    setTimeout(() => { showCopiedToast.value = false; }, 3000);
}

loadTracker();
</script>

<template>
    <div v-if="loading && !trackerData" class="text-center">Loading tracker data...</div>
    <div v-if="error && !trackerData" class="text-center text-danger">Failed to load tracker data ({{ error.message }})
    </div>
    <template v-if="trackerData">
        <div v-if="!settings.discordUsername?.length" class="alert alert-info text-center">
            You have not set your Discord username in the <router-link to="/settings">settings</router-link>. You will be
            unable to claim slots until you do this.
        </div>
        <button class="btn btn-primary refresh-button" @click="loadTracker()" :disabled="loading">Refresh</button>
        <table class="table table-sm table-hover text-center">
            <thead style="position: sticky; top: 0; z-index: 100">
                <tr class="tracker-header">
                    <th>Name</th>
                    <th>Ping</th>
                    <th></th>
                    <th>
                        <div class="dropdown">
                            <button class="btn btn-sm btn-outline-light dropdown-toggle" data-bs-toggle="dropdown">
                                Owner (Discord Username)
                            </button>
                            <ul class="dropdown-menu">
                                <li>
                                    <button class="dropdown-item" :class="{ active: playerFilter === PLAYER_FILTER_ALL }"
                                        @click="playerFilter = PLAYER_FILTER_ALL">
                                        All
                                    </button>
                                </li>
                                <li>
                                    <button class="dropdown-item"
                                        :class="{ active: playerFilter === PLAYER_FILTER_UNOWNED }"
                                        @click="playerFilter = PLAYER_FILTER_UNOWNED">
                                        Unset
                                    </button>
                                </li>
                                <template v-if="settings.discordUsername?.length">
                                    <li>
                                        <hr class="dropdown-divider">
                                    </li>
                                    <button class="dropdown-item"
                                        :class="{ active: playerFilter === settings.discordUsername }"
                                        @click="playerFilter = settings.discordUsername">
                                        {{ settings.discordUsername }}
                                    </button>
                                </template>
                                <template v-if="playersExceptSelf.length">
                                    <li>
                                        <hr class="dropdown-divider">
                                    </li>
                                    <button v-for="player in playersExceptSelf" class="dropdown-item"
                                        :class="{ active: playerFilter === player }" @click="playerFilter = player">
                                        {{ player }}
                                    </button>
                                </template>
                            </ul>
                        </div>
                    </th>
                    <th>Game</th>
                    <th>
                        <div class="dropdown">
                            <button class="btn btn-sm btn-outline-light dropdown-toggle" data-bs-toggle="dropdown"
                                data-bs-auto-close="outside">
                                Status
                            </button>
                            <ul class="dropdown-menu">
                                <li v-for="status in statuses">
                                    <button class="dropdown-item" :class="{
                                        active: statusFilter[status],
                                        'text-black': statusFilter[status],
                                        [`bg-${gameStatus.byId[status].color}`]: statusFilter[status],
                                        [`text-${gameStatus.byId[status].color}`]: !statusFilter[status]
                                    }" @click="statusFilter[status] = !statusFilter[status]">
                                        {{ gameStatus.byId[status].label }}
                                    </button>
                                </li>
                            </ul>
                        </div>
                    </th>
                    <th colspan="2">Last Activity (Days)</th>
                    <th>Checks</th>
                    <th>
                        <div :class="{ dropdown: !allExpanded, dropup: allExpanded }">
                            <button class="btn btn-sm btn-outline-light dropdown-toggle"
                                @click="setAllExpanded(!allExpanded)">Hints</button>
                        </div>
                    </th>
                </tr>
            </thead>
            <tbody>
                <template v-for="game in  filteredGames ">
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
                                :class="[`btn-outline-${gameStatus.byId[game.status].color}`]" data-bs-toggle="dropdown">
                                {{ gameStatus.byId[game.status].label }}
                            </button>
                            <ul class="dropdown-menu">
                                <li v-for="status in statuses">
                                    <button class="dropdown-item" :class="[`text-${gameStatus.byId[status].color}`]"
                                        :disabled="loading || status === game.status"
                                        @click="setGameStatus(game, status)">{{
                                            gameStatus.byId[status].label }}</button>
                                </li>
                            </ul>
                        </td>
                        <td :class="lastCheckedClass(game)" :title="displayDateTime(gameLastUpdated(game))">{{
                            displayLastChecked(game) }}</td>
                        <td>
                            <button class=" btn btn-sm btn-outline-secondary" :class="{ invisible: game.status === 'done' }"
                                :disabled="loading" @click="updateLastChecked(game)">Update</button>
                        </td>
                        <td class="align-middle">
                            <ChecksBar :done="game.checks_done" :total="game.checks_total"></ChecksBar>
                        </td>
                        <td>
                            <div :class="{ dropdown: gameExpanded[game.id], dropup: gameExpanded[game.id] }">
                                <button class="btn btn-sm dropdown-toggle" :class="[hintsClass(game)]"
                                    @click="gameExpanded[game.id] = !gameExpanded[game.id]">
                                    {{ unfoundHints(game) }}<template v-if="game.notes !== ''">*</template>
                                </button>
                            </div>
                        </td>
                    </tr>
                    <tr v-if="gameExpanded[game.id]">
                        <td colspan="11" class="container-fluid">
                            <div class="row">
                                <div class="col-12 col-xl-6">
                                    <div>
                                        <div class="btn-group">
                                            <button class="btn btn-sm btn-outline-light" :class="{ active: !sentHints }"
                                                @click="sentHints = false">
                                                Received hints
                                            </button>
                                            <button class="btn btn-sm btn-outline-light" :class="{ active: sentHints }"
                                                @click="sentHints = true">
                                                Sent hints
                                            </button>
                                        </div>
                                        <button class="btn btn-sm btn-outline-light ms-2"
                                            :disabled="unfoundHintsByGame(game.id).length === 0"
                                            @click="copyHints(unfoundHintsByGame(game.id))">Copy all</button>
                                    </div>
                                    <div v-if="unfoundHintsByGame(game.id).length === 0" class="text-muted">
                                        There are no unfound hints right now.
                                    </div>
                                    <div v-else class="row justify-content-center">
                                        <div class="col-auto">
                                            <table class="table table-responsive">
                                                <tr v-for="hint in unfoundHintsByGame(game.id)">
                                                    <td class="text-end pe-0">
                                                        <template v-if="!sentHints">
                                                            <span class="text-info bg-transparent p-0">{{
                                                                gameById[hint.receiver_game_id].name
                                                            }}</span>'s
                                                        </template>
                                                        <span class="text-info bg-transparent p-0">{{ hint.item }}</span>
                                                    </td>
                                                    <td class="ps-0 pe-0">&nbsp;is&nbsp;at&nbsp;</td>
                                                    <td class="text-start ps-0">
                                                        <template v-if="sentHints">
                                                            <span class="text-info bg-transparent p-0">{{
                                                                gameById[hint.finder_game_id].name
                                                            }}</span>'s
                                                        </template>
                                                        <span class="text-info bg-transparent p-0">{{ hint.location
                                                        }}</span>
                                                        <template v-if="hint.entrance !== 'Vanilla'"> ({{ hint.entrance
                                                        }})</template> <a href="#"
                                                            class="bg-transparent p-0 mw-copy-hint"
                                                            @click.prevent="clipboardCopy(hintToString(hint))"
                                                            title="Copy to clipboard">&#x1F4C4;</a>
                                                    </td>
                                                </tr>
                                            </table>
                                        </div>
                                    </div>
                                </div>
                                <div class="col-12 col-xl-6">
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
                                <td>{{ players.length }}</td>
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
        <div class="row">
            <div class="col-12 col-lg-6">
                <TrackerSummary :tracker-data="trackerData" summarize-by="discord_username"></TrackerSummary>
            </div>
            <div class="col-12 col-lg-6">
                <TrackerSummary :tracker-data="trackerData" summarize-by="game"></TrackerSummary>
            </div>
        </div>
        <div class="text-center">Last updated from Archipelago at {{ displayDateTime(trackerData.updated_at) }}
        </div>
    </template>
    <div class="toast-container position-fixed bottom-0 end-0 p-3">
        <div class="toast text-bg-success" :class="{ show: showCopiedToast }">
            <div class="toast-body">
                Copied to the clipboard.
            </div>
        </div>
    </div>
</template>

<style scoped>
.refresh-button {
    position: fixed;
    bottom: 1rem;
    right: 1.5rem;
    z-index: 1;
}

.mw-copy-hint {
    visibility: hidden;
    text-decoration: none;
}

tr tr:hover .mw-copy-hint {
    visibility: visible;
}

.tracker-header th {
    vertical-align: middle;
}
</style>