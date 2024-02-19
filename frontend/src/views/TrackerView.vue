<script setup>
import { computed, onUnmounted, ref, watch } from 'vue';
import { groupBy, keyBy, orderBy, sumBy, uniq, mapValues, map, filter, reduce, join, includes } from 'lodash-es';
import moment from 'moment';
import { settings } from '@/settings';
import { now } from '@/time';
import { getTracker as apiGetTracker, updateGame as apiUpdateGame, updateTracker as apiUpdateTracker } from '@/api';
import { gameStatus, pingPreference } from '@/types';
import { percent, synchronize } from '@/util';
import TrackerSummary from '@/components/TrackerSummary.vue';
import ChecksBar from '@/components/ChecksBar.vue';

const props = defineProps(['aptrackerid']);

const loading = ref(false);
const error = ref(undefined);
const trackerData = ref(undefined);
const hintsByFinder = ref(undefined);
const hintsByReceiver = ref(undefined);
const gameById = ref(undefined);

watch(
    () => trackerData.value?.title,
    title => {
        if (title) {
            document.title = `${title} | Cheese Trackers`;
        } else {
            document.title = 'Cheese Trackers';
        }
    }
);

onUnmounted(() => {
    document.title = 'Cheese Trackers';
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
    const unfound = Math.max(countUnfoundReceivedHints(game), game.notes !== '' ? 1 : 0);

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
        ),
        i => i.toLowerCase()
    )
);

const playersExceptSelf = computed(() =>
    filter(players.value, p => p !== settings.value.discordUsername)
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

function gameDaysSinceLastChecked(game) {
    const lastUpdated = gameLastUpdated(game);

    return lastUpdated && Math.max(0, moment.duration(moment.utc(now.value).diff(lastUpdated)).asDays());
}

function isGameCompleted(game) {
    return includes(['done', 'released', 'glitched'], game.status);
}

function lastCheckedClass(game) {
    if (isGameCompleted(game)) {
        return 'text-success';
    }

    const sinceDays = gameDaysSinceLastChecked(game);

    if (sinceDays === undefined) {
        return 'text-danger';
    }

    for (const t of lastCheckedThresholds) {
        if (t.days === undefined || sinceDays >= t.days) {
            return `text-${t.color}`;
        }
    }
}

function displayLastChecked(game) {
    const days = gameDaysSinceLastChecked(game);

    return days === undefined ? 'Never' : days.toFixed(1);
}

const statGames = computed((() => {
    const excludedStatuses = ['released', 'glitched'];
    return () => filter(trackerData.value?.games, g => !includes(excludedStatuses, g.status));
})());

const statUniqueGames = computed(() =>
    uniq(statGames.value.map(g => g.game)).length
);

const statTotalDoneChecks = computed(() =>
    sumBy(statGames.value, 'checks_done')
);

const statTotalChecks = computed(() =>
    sumBy(statGames.value, 'checks_total')
);

const statGamesByStatus = computed(() =>
    groupBy(statGames.value, 'status')
);

function statChecksByStatusProgression(status) {
    if (status === 'done') {
        return sumBy(statGames.value, 'checks_done');
    }

    return sumBy(statGamesByStatus.value[status], g => g.checks_total - g.checks_done);
}

const statuses = gameStatus.map(i => i.id);
const statusFilter = ref(mapValues(gameStatus.byId, () => true));

const statusFilterActive = computed(() =>
    !statuses.every(s => statusFilter.value[s])
);

const filteredGames = computed(() =>
    filter(trackerData.value?.games, g =>
        (
            playerFilter.value === PLAYER_FILTER_ALL ? true :
                playerFilter.value === PLAYER_FILTER_UNOWNED ? !g.discord_username?.length :
                    playerFilter.value === g.discord_username
        ) &&
        statusFilter.value[g.status]
    )
);

function sortByName(g) {
    return g.name.toLowerCase();
}

function sortByActivity(g) {
    const days = gameDaysSinceLastChecked(g);
    return days === undefined ? Number.POSITIVE_INFINITY : days;
}

const activeSort = ref([sortByName, false]);

function setSort(sorter, defOrder) {
    if (activeSort.value[0] === sorter) {
        activeSort.value[1] = !activeSort.value[1];
    } else {
        activeSort.value = [sorter, defOrder];
    }
}

const sortedAndFilteredGames = computed(() =>
    orderBy(filteredGames.value, activeSort.value[0], activeSort.value[1] ? 'desc' : 'asc')
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
const showFoundHints = ref(false);

const hintsByGame = computed(() => {
    return sentHints.value ? hintsByReceiver.value : hintsByFinder.value;
})

function hintStatus(hint) {
    return hint.found ? 'found' :
        (
            hint.receiver_game_id !== undefined &&
            gameById.value[hint.receiver_game_id].status === 'done'
        ) ? 'useless' :
            'notfound';
}

const HINT_STATUS_UI = {
    found: {
        iconclasses: ['bi-check-circle-fill', 'text-success'],
        icontooltip: 'Found',
        rowclasses: ['bg-success-subtle'],
    },
    notfound: {
        iconclasses: ['bi-x-circle-fill', 'text-danger'],
        icontooltip: 'Not found',
        rowclasses: ['bg-danger-subtle'],
    },
    useless: {
        iconclasses: ['bi-x-circle-fill', 'text-info'],
        icontooltip: 'Not found, receiving slot is done',
        rowclasses: ['bg-info-subtle'],
    },
}

function displayHintsByGame(id) {
    return (hintsByGame.value?.[id] || []).filter(h => showFoundHints.value || hintStatus(h) === 'notfound');
}

function countUnfoundReceivedHints(game) {
    return (hintsByFinder.value[game.id] || []).filter(h => hintStatus(h) === 'notfound').length;
}

function patchGame(game) {
    game.$newnotes = game.notes;
}

async function loadTracker() {
    if (loading.value) {
        return;
    }

    loading.value = true;
    error.value = undefined;

    try {
        const { data } = await apiGetTracker(props.aptrackerid);
        data.games.forEach(patchGame);
        trackerData.value = data;

        hintsByFinder.value = groupBy(trackerData.value.hints, 'finder_game_id');
        hintsByReceiver.value = groupBy(filter(trackerData.value.hints, h => h.receiver_game_id !== undefined), 'receiver_game_id');
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
            ({ status, data }) => status >= 200 && status < 300 ? data : undefined,
            e => {
                console.error(`Failed to update game: ${e}`);
                return undefined;
            }
        )
        .then(savedGame => {
            if (savedGame) {
                synchronize(game, savedGame);
                patchGame(game);
            }
            loading.value = false;
        });
}

function claimGame(game) {
    updateGame(game, g => {
        g.discord_username = settings.value.discordUsername;
        g.discord_ping = settings.value.defaultPingPreference;
    });
}

function unclaimGame(game) {
    updateGame(game, g => {
        delete g.discord_username;
        g.discord_ping = 'never';
    });
}

function setGameStatus(game, status) {
    const nowStr = (new Date()).toISOString();

    // HACK: We use setTimeout here because otherwise the dropdown becomes
    // disabled before Bootstrap closes the dropdown.  When it tries to do so,
    // it finds it disabled and won't close it.
    setTimeout(() => updateGame(game, g => {
        g.status = status;

        // If setting to BK then also update "last checked."
        if (status === 'bk') {
            g.last_checked = nowStr;
        }
    }));
}

function setPing(game, preference) {
    setTimeout(() => updateGame(game, g => { g.discord_ping = preference; }));
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
    const receiver = gameById.value[hint.receiver_game_id]?.name || '(Item link)';
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

const editedTitle = ref('');
const editingTitle = ref(false);
const editTitleInput = ref(undefined);

function editTitle() {
    editedTitle.value = trackerData.value.title || '';
    editingTitle.value = true;
    setTimeout(() => { editTitleInput.value?.focus(); });
}

function saveTitle() {
    editingTitle.value = false;

    if ((trackerData.value.title || '') !== editedTitle.value) {
        if (loading.value) {
            return;
        }

        loading.value = true;

        const newTitle = editedTitle.value;
        apiUpdateTracker({
            tracker_id: trackerData.value.tracker_id,
            title: newTitle,
        })
            .then(({ status }) => status >= 200 && status < 300,
                e => {
                    console.error(`Failed to update tracker: ${e}`);
                    return false;
                })
            .then(saved => {
                loading.value = false;
                if (saved) {
                    trackerData.value.title = newTitle;
                }
            })
    }
}

function cancelEditTitle() {
    editedTitle.value = trackerData.value.title;
    editingTitle.value = false;
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
        <h2 v-if="!editingTitle" @click="editTitle" class="text-center mb-4"
            :class="{ 'text-muted': !trackerData.title.length, 'fst-italic': !trackerData.title.length }">{{
                trackerData.title.length ?
                trackerData.title : 'Untitled tracker' }}</h2>
        <input v-if="editingTitle" ref="editTitleInput" class="form-control text-center" placeholder="Title"
            v-model="editedTitle" @blur="saveTitle" @keyup.enter="saveTitle" @keyup.esc="cancelEditTitle">
        <button class="btn btn-primary refresh-button" @click="loadTracker()" :disabled="loading">Refresh</button>
        <table class="table table-sm table-hover text-center tracker-table">
            <thead style="position: sticky; top: 0; z-index: 100">
                <tr>
                    <th class="sortable-column" @click="setSort(sortByName, false)">
                        Name
                        <i v-if="activeSort[0] === sortByName"
                            :class="{ 'bi-sort-alpha-down': !activeSort[1], 'bi-sort-alpha-up': activeSort[1] }"></i>
                    </th>
                    <th>Ping</th>
                    <th></th>
                    <th>
                        <div class="dropdown">
                            <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown">
                                Owner (Discord Username) <i
                                    :class="{ 'bi-funnel': playerFilter === PLAYER_FILTER_ALL, 'bi-funnel-fill': playerFilter !== PLAYER_FILTER_ALL }"></i>
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
                            <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown"
                                data-bs-auto-close="outside">
                                Status <i
                                    :class="{ 'bi-funnel': !statusFilterActive, 'bi-funnel-fill': statusFilterActive }"></i>
                            </button>
                            <ul class="dropdown-menu">
                                <li v-for="status in statuses">
                                    <button class="dropdown-item" :class="{
                                        active: statusFilter[status],
                                        [`bg-${gameStatus.byId[status].color}`]: statusFilter[status],
                                        [`text-bg-${gameStatus.byId[status].color}`]: statusFilter[status],
                                        [`text-${gameStatus.byId[status].color}`]: !statusFilter[status]
                                    }" @click="statusFilter[status] = !statusFilter[status]">
                                        {{ gameStatus.byId[status].label }}
                                    </button>
                                </li>
                            </ul>
                        </div>
                    </th>
                    <th colspan="2" class="sortable-column" @click="setSort(sortByActivity, true)">
                        Last Activity (Days)
                        <i v-if="activeSort[0] === sortByActivity"
                            :class="{ 'bi-sort-numeric-down': !activeSort[1], 'bi-sort-numeric-up': activeSort[1] }"></i>
                    </th>
                    <th>Checks</th>
                    <th>
                        <button class="btn btn-sm btn-outline-light" @click="setAllExpanded(!allExpanded)">Hints <i
                                :class="{ 'bi-arrows-angle-expand': !allExpanded, 'bi-arrows-angle-contract': allExpanded }"></i></button>
                    </th>
                </tr>
            </thead>
            <tbody>
                <template v-for="game in sortedAndFilteredGames">
                    <tr>
                        <td>
                            <a :href="`https://archipelago.gg/tracker/${trackerData.tracker_id}/0/${game.position}`"
                                target="_blank" class="text-reset mw-underline-hover">{{
                                    game.name }}</a>
                        </td>
                        <td>
                            <span v-if="game.discord_username && game.status === 'done'" class="text-danger">Never</span>
                            <button v-else-if="game.discord_username" class="btn btn-sm dropdown-toggle" :disabled="loading"
                                :class="[`btn-outline-${pingPreference.byId[game.discord_ping].color}`]"
                                data-bs-toggle="dropdown">
                                {{ pingPreference.byId[game.discord_ping].label }}
                            </button>
                            <ul class="dropdown-menu">
                                <li v-for="pref in pingPreference">
                                    <button class="dropdown-item" :class="[`text-${pref.color}`]"
                                        :disabled="loading || pref.id === game.discord_ping"
                                        @click="setPing(game, pref.id)">{{ pref.label }}</button>
                                </li>
                            </ul>
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
                        <td class="text-end" :class="lastCheckedClass(game)">
                            <span :title="displayDateTime(gameLastUpdated(game))">{{
                                displayLastChecked(game) }}</span>
                        </td>
                        <td class="text-start">
                            <button class=" btn btn-sm btn-outline-secondary" :class="{ invisible: game.status !== 'bk' }"
                                :disabled="loading" @click="updateLastChecked(game)">Still BK</button>
                        </td>
                        <td>
                            <ChecksBar :done="game.checks_done" :total="game.checks_total"></ChecksBar>
                        </td>
                        <td>
                            <button class="btn btn-sm" :class="[hintsClass(game)]"
                                @click="gameExpanded[game.id] = !gameExpanded[game.id]">
                                {{ countUnfoundReceivedHints(game) }}<template v-if="game.notes !== ''">*</template> <i
                                    :class="{ 'bi-arrows-angle-expand': !gameExpanded[game.id], 'bi-arrows-angle-contract': gameExpanded[game.id] }"></i>
                            </button>
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
                                        <button class="btn btn-sm ms-2 btn-outline-light"
                                            :class="{ active: showFoundHints }" @click="showFoundHints = !showFoundHints">
                                            Include found and useless hints
                                        </button>
                                        <button class=" btn btn-sm btn-outline-light ms-2"
                                            :disabled="displayHintsByGame(game.id).length === 0"
                                            @click="copyHints(displayHintsByGame(game.id))">Copy all</button>
                                    </div>
                                    <div v-if="displayHintsByGame(game.id).length === 0" class="text-muted">
                                        There are no unfound hints right now.
                                    </div>
                                    <div v-else class="row justify-content-center">
                                        <div class="col-auto">
                                            <table class="table table-responsive">
                                                <tr v-for="hint in displayHintsByGame(game.id)">
                                                    <td class="text-end pe-0">
                                                        <template v-if="!sentHints">
                                                            <span class="bg-transparent p-0"
                                                                :class="{ 'text-info': !!gameById[hint.receiver_game_id], 'text-primary': !gameById[hint.receiver_game_id] }">{{
                                                                    gameById[hint.receiver_game_id]?.name || '(Item link)'
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
                                                        }})</template> <i v-if="showFoundHints" class="bg-transparent"
                                                            :class="HINT_STATUS_UI[hintStatus(hint)].iconclasses"
                                                            :title="HINT_STATUS_UI[hintStatus(hint)].icontooltip"></i> <a
                                                            href="#" class="bg-transparent p-0 mw-copy-hint"
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
                                <th>Total checks</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td>{{ players.length }}</td>
                                <td>{{ statUniqueGames }}</td>
                                <td class="align-middle">
                                    <ChecksBar :done="statTotalDoneChecks" :total="statTotalChecks" show-percent="1">
                                    </ChecksBar>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                    <table class="table table-sm">
                        <thead>
                            <tr>
                                <th class="text-center" colspan="2">Status summary</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <th class="text-end shrink-column">Progression</th>
                                <td class="align-middle">
                                    <div class="progress">
                                        <div v-for="status in statuses" class="progress-bar"
                                            :class="[`bg-${gameStatus.byId[status].color}`]"
                                            :style="{ width: `${percent(statChecksByStatusProgression(status), statTotalChecks)}%` }">
                                        </div>
                                    </div>
                                </td>
                            </tr>
                            <!--
                            <tr>
                                <th class="text-end shrink-column">By total checks</th>
                                <td class="align-middle">
                                    <div class="progress">
                                        <div v-for="status in statuses" class="progress-bar"
                                            :class="[`bg-${gameStatus.byId[status].color}`]"
                                            :style="{ width: `${percent(sumBy(statGamesByStatus[status], 'checks_total'), statTotalChecks)}%` }">
                                        </div>
                                    </div>
                                </td>
                            </tr>
                            -->
                            <tr>
                                <th class="text-end shrink-column">By slot</th>
                                <td class="align-middle">
                                    <div class="progress">
                                        <div v-for="status in statuses" class="progress-bar"
                                            :class="[`bg-${gameStatus.byId[status].color}`]"
                                            :style="{ width: `${percent(statGamesByStatus[status]?.length || 0, statGames.length)}%` }">
                                        </div>
                                    </div>
                                </td>
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

.tracker-table th,
.tracker-table td {
    vertical-align: middle;
}

.shrink-column {
    width: 1px;
    white-space: nowrap;
}

.sortable-column {
    cursor: pointer;
}

.mw-underline-hover {
    text-decoration: none;
}

.mw-underline-hover:hover {
    text-decoration: underline;
}
</style>