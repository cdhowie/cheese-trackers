<script setup>
// This view could use some refactoring to break stuff out into components.

import { computed, onUnmounted, ref, watch } from 'vue';
import { groupBy, keyBy, orderBy, sumBy, uniq, map, filter, reduce, join, includes, uniqBy, isEqual, fromPairs, every, omit } from 'lodash-es';
import moment from 'moment';
import { settings } from '@/settings';
import { now } from '@/time';
import { getTracker as apiGetTracker, updateGame as apiUpdateGame, updateTracker as apiUpdateTracker } from '@/api';
import { progressionStatus, completionStatus, availabilityStatus, pingPreference } from '@/types';
import { percent, synchronize } from '@/util';
import TrackerSummary from '@/components/TrackerSummary.vue';
import ChecksBar from '@/components/ChecksBar.vue';
import UsernameDisplay from '@/components/UsernameDisplay.vue';
import GameDisplay from '@/components/GameDisplay.vue';
import DropdownSelector from '@/components/DropdownSelector.vue';

const props = defineProps(['aptrackerid']);

const loading = ref(false);
const error = ref(undefined);
const trackerData = ref(undefined);
const hintsByFinder = ref(undefined);
const hintsByReceiver = ref(undefined);
const gameById = ref(undefined);

// Hack that should probably exist as a global service.
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

const currentUser = computed(() =>
    settings.value.auth?.token ? {
        id: settings.value.auth.userId,
        discordUsername: settings.value.auth.discordUsername,
    } : settings.value.unauthenticatedDiscordUsername ? {
        discordUsername: settings.value.unauthenticatedDiscordUsername,
    } : undefined
);

const trackerOwner = computed(() =>
    trackerData.value.owner_ct_user_id && {
        id: trackerData.value.owner_ct_user_id,
        discordUsername: trackerData.value.owner_discord_username,
    }
);

const currentUserIsTrackerOwner = computed(() => {
    const uid = currentUser.value?.id;

    return uid !== undefined && uid === trackerOwner.value?.id;
});

function updateTracker(data) {
    if (loading.value) {
        return;
    }

    loading.value = true;

    apiUpdateTracker(Object.assign(
        omit(trackerData.value, 'games', 'hints'),
        data
    ))
        .then(
            ({ status }) => status >= 200 && status < 300,
            e => {
                console.error(`Failed to update tracker: ${e}`);
                return false;
            }
        )
        .then(saved => {
            loading.value = false;
            if (saved) {
                Object.assign(trackerData.value, data);
            }
        })
}

function claimTracker() {
    updateTracker({
        owner_ct_user_id: currentUser.value.id,
        // This will be ignored by the server, but is used to update our local
        // state after the request.
        owner_discord_username: currentUser.value.discordUsername,
    });
}

function disclaimTracker() {
    updateTracker({
        owner_ct_user_id: undefined,
        owner_discord_username: undefined,
    });
}

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

function getClaimingUser(game) {
    if (game.claimed_by_ct_user_id !== undefined) {
        return {
            id: game.claimed_by_ct_user_id,
            discordUsername: game.discord_username,
        };
    }

    if (game.discord_username?.length) {
        return { discordUsername: game.discord_username };
    }

    return undefined;
}

const players = computed(() =>
    orderBy(
        uniqBy(
            filter(map(trackerData.value.games, getClaimingUser)),
            u => u.id !== undefined ? u.id : u.discordUsername
        ),
        i => i.discordUsername.toLowerCase()
    )
);

const playersExceptSelf = computed(() =>
    currentUser.value ?
        filter(players.value, p => !isEqual(p, currentUser.value)) :
        players.value
);

const PLAYER_FILTER_ALL = Symbol();
const PLAYER_FILTER_UNOWNED = Symbol();

const playerFilter = ref(PLAYER_FILTER_ALL);

const uniqueGames = computed(() =>
    orderBy(
        uniq(
            map(trackerData.value.games, 'game')
        ),
        i => i.toLowerCase()
    )
);

const gameFilter = ref(undefined);

const showLastActivity = ref(false);

// TODO: Make these configurable by the room owner.
const lastCheckedThresholds = [
    { days: 2, color: 'danger' },
    { days: 1, color: 'warning' },
    { color: 'success' }
];

function getLastCheckedOrLastActivity(game) {
    return reduce(
        map(
            filter([game.last_checked, game.last_activity]),
            d => moment.utc(d)
        ),
        (a, b) => moment.max(a, b)
    );
}

function gameDaysSinceLastCheckedOrLastActivity(game) {
    const lastUpdated = getLastCheckedOrLastActivity(game);

    return lastUpdated && Math.max(0, dateToDays(lastUpdated));
}

function dateToDays(d) {
    if (!moment.isMoment(d)) {
        d = moment.utc(d);
    }

    return moment.duration(moment.utc(now.value).diff(d)).asDays();
}

function isGameCompleted(game) {
    return includes(['done', 'released'], game.completion_status);
}

function lastCheckedClass(game) {
    if (isGameCompleted(game)) {
        return 'text-success';
    }

    const sinceDays = gameDaysSinceLastCheckedOrLastActivity(game);

    if (sinceDays === undefined) {
        return 'text-danger';
    }

    for (const t of lastCheckedThresholds) {
        if (t.days === undefined || sinceDays >= t.days) {
            return `text-${t.color}`;
        }
    }
}

function displayLastCheckedOrLastActivity(game) {
    const days = gameDaysSinceLastCheckedOrLastActivity(game);

    return days === undefined ? 'Never' : days.toFixed(1);
}

function displayLastActivity(game) {
    const d = game.last_activity;

    return d === undefined ? 'Never' : Math.max(0, dateToDays(d)).toFixed(1);
}

const statGames = computed(() =>
    filter(trackerData.value?.games, g => g.completion_status !== 'released')
);

const statUniqueGames = computed(() =>
    uniq(statGames.value.map(g => g.game)).length
);

const statTotalDoneChecks = computed(() =>
    sumBy(statGames.value, 'checks_done')
);

const statTotalChecks = computed(() =>
    sumBy(statGames.value, 'checks_total')
);

const statGamesByCompletionStatus = computed(() =>
    groupBy(
        filter(
            statGames.value,
            g => g.completion_status !== 'incomplete' || g.progression_status !== 'bk'
        ),
        'completion_status'
    )
);

const statGamesByProgressionStatus = computed(() =>
    groupBy(statGames.value, 'progression_status')
);

function statChecksByStatusProgression(status) {
    return sumBy(statGamesByProgressionStatus.value[status], g => g.checks_total - g.checks_done);
}

function makeStatusFilter(type, gameKey) {
    const types = ref(fromPairs(map(type, t => [t.id, true])));
    const isActive = computed(() =>
        !every(type, t => types.value[t.id])
    );
    function showGame(g) {
        return types.value[g[gameKey]];
    }
    function toggle(status) {
        status = status.id || status;
        types.value[status] = !types.value[status];
    }
    function classes(status) {
        return types.value[status.id] ?
            ['active', `bg-${status.color}`, `text-bg-${status.color}`] :
            [`text-${status.color}`];
    }

    return { types, isActive, showGame, toggle, classes };
}

const completionFilter = makeStatusFilter(completionStatus, 'completion_status');
const availabilityFilter = makeStatusFilter(availabilityStatus, 'availability_status');

// Progression filters do not apply to completed games.  To keep
// makeStatusFilter simple, we'll just patch that function for this one filter.
const progressionFilter = (() => {
    const f = makeStatusFilter(progressionStatus, 'progression_status');
    const showGame = f.showGame;
    f.showGame = g => isGameCompleted(g) || showGame(g);
    return f;
})();

const filteredGames = computed(() =>
    filter(trackerData.value?.games, g => {
        const user = getClaimingUser(g);

        return every(
            [progressionFilter, completionFilter, availabilityFilter], f => f.showGame(g)
        ) && (
                playerFilter.value === PLAYER_FILTER_ALL ? true :
                    playerFilter.value === PLAYER_FILTER_UNOWNED ? !user :
                        isEqual(playerFilter.value, user)
            ) && (
                gameFilter.value === undefined ||
                gameFilter.value === g.game
            );
    })
);

function sortByName(g) {
    return g.name.toLowerCase();
}

function sortByGame(g) {
    return g.game.toLowerCase();
}

function sortByActivity(g) {
    const days = gameDaysSinceLastCheckedOrLastActivity(g);
    return days === undefined ? Number.POSITIVE_INFINITY : days;
}

function sortByOwner(g) {
    return (g.discord_username || '').toLowerCase();
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
            gameById.value[hint.receiver_game_id].completion_status === 'done'
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
        if (settings.value.auth?.token) {
            g.claimed_by_ct_user_id = settings.value.auth.userId;
        } else {
            g.discord_username = settings.value.unauthenticatedDiscordUsername;
        }

        if (includes(['unknown', 'open'], g.availability_status)) {
            g.availability_status = 'claimed';
        }

        g.discord_ping = settings.value.defaultPingPreference;
    });
}

function unclaimGame(game) {
    updateGame(game, g => {
        delete g.claimed_by_ct_user_id;
        delete g.discord_username;

        if (g.availability_status === 'claimed') {
            g.availability_status = 'open';
        }

        g.discord_ping = 'never';
    });
}

function setGameProgressionStatus(game, status) {
    status = status.id || status;
    const nowStr = (new Date()).toISOString();

    // HACK: We use setTimeout here because otherwise the dropdown becomes
    // disabled before Bootstrap closes the dropdown.  When it tries to do so,
    // it finds it disabled and won't close it.
    setTimeout(() => updateGame(game, g => {
        g.progression_status = status;

        // If setting to BK then also update "last checked."
        if (status === 'bk') {
            g.last_checked = nowStr;
        }
    }));
}

function setGameCompletionStatus(game, status) {
    setTimeout(() => updateGame(game, g => {
        g.completion_status = status.id || status;
    }))
}

function setGameAvailabilityStatus(game, status) {
    setTimeout(() => updateGame(game, g => {
        g.availability_status = status.id || status;
    }));
}

function setPing(game, preference) {
    setTimeout(() => updateGame(game, g => {
        g.discord_ping = preference.id || preference;
    }));
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
        updateTracker({ title: editedTitle.value });
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
        <div v-if="!currentUser" class="alert alert-info text-center">
            You will be unable to claim slots until you either sign in with Discord or set your Discord username in the
            <router-link to="/settings">settings</router-link>.
        </div>
        <h2 v-if="!editingTitle" @click="editTitle" class="text-center mb-4"
            :class="{ 'text-muted': !trackerData.title.length, 'fst-italic': !trackerData.title.length }">{{
                trackerData.title.length ?
                trackerData.title : 'Untitled tracker' }}</h2>
        <input v-if="editingTitle" ref="editTitleInput" class="form-control text-center" placeholder="Title"
            v-model="editedTitle" @blur="saveTitle" @keyup.enter="saveTitle" @keyup.esc="cancelEditTitle">
        <div class="text-center">
            Organizer: <UsernameDisplay :user="trackerOwner"></UsernameDisplay> <button
                v-if="currentUser?.id !== undefined && !trackerOwner" class="btn btn-sm btn-outline-secondary"
                :disabled="loading" @click="claimTracker">Claim</button>

            <button v-if="currentUserIsTrackerOwner" class="btn btn-sm btn-outline-warning" :disabled="loading"
                @click="disclaimTracker">Disclaim</button>
        </div>
        <button class="btn btn-primary refresh-button" @click="loadTracker()" :disabled="loading">Refresh</button>
        <table class="table table-sm table-hover text-center tracker-table">
            <thead style="position: sticky; top: 0; z-index: 100">
                <tr>
                    <th @click="setSort(sortByName, false)">
                        <span class="sorter">
                            Name
                            <i v-if="activeSort[0] === sortByName"
                                :class="{ 'bi-sort-alpha-down': !activeSort[1], 'bi-sort-alpha-up': activeSort[1] }"></i>
                        </span>
                    </th>
                    <th>Ping</th>
                    <th colspan="2">
                        Availability
                        <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown" data-bs-auto-close="outside">
                            <i :class="[availabilityFilter.isActive.value ? 'bi-funnel-fill' : 'bi-funnel']"></i>
                        </button>
                        <ul class="dropdown-menu">
                            <li v-for="status in availabilityStatus">
                                <button class="dropdown-item" :class="availabilityFilter.classes(status)"
                                    @click="availabilityFilter.toggle(status)">
                                    {{ status.label }}
                                </button>
                            </li>
                        </ul>
                    </th>
                    <th>
                        <div class="dropdown">
                            <span @click="setSort(sortByOwner, false)" class="sorter">
                                Owner (Discord Username)
                                <i v-if="activeSort[0] === sortByOwner" class="me-1"
                                    :class="{ 'bi-sort-alpha-down': !activeSort[1], 'bi-sort-alpha-up': activeSort[1] }"></i>
                            </span>
                            <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown">
                                <i
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
                                        Unclaimed
                                    </button>
                                </li>
                                <template v-if="currentUser">
                                    <li>
                                        <hr class="dropdown-divider">
                                    </li>
                                    <li>
                                        <button class="dropdown-item"
                                            :class="{ active: isEqual(playerFilter, currentUser) }"
                                            @click="playerFilter = currentUser">
                                            <UsernameDisplay :user="currentUser"></UsernameDisplay>
                                        </button>
                                    </li>
                                </template>
                                <template v-if="playersExceptSelf.length">
                                    <li>
                                        <hr class="dropdown-divider">
                                    </li>
                                    <li>
                                        <button v-for="player in playersExceptSelf" class="dropdown-item"
                                            :class="{ active: playerFilter === player }" @click="playerFilter = player">
                                            <UsernameDisplay :user="player"></UsernameDisplay>
                                        </button>
                                    </li>
                                </template>
                            </ul>
                        </div>
                    </th>
                    <th>
                        <div class="dropdown">
                            <span @click="setSort(sortByGame, false)" class="sorter">
                                Game
                                <i v-if="activeSort[0] === sortByGame" class="me-1"
                                    :class="{ 'bi-sort-alpha-down': !activeSort[1], 'bi-sort-alpha-up': activeSort[1] }"></i>
                            </span>
                            <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown">
                                <i :class="{ 'bi-funnel': !gameFilter, 'bi-funnel-fill': !!gameFilter }"></i>
                            </button>
                            <ul class="dropdown-menu">
                                <li>
                                    <button class="dropdown-item" :class="{ active: !gameFilter }"
                                        @click="gameFilter = undefined">All</button>
                                </li>
                                <li>
                                    <hr class="dropdown-divider">
                                </li>
                                <li v-for="g in uniqueGames">
                                    <button class="dropdown-item" :class="{ active: gameFilter === g }"
                                        @click="gameFilter = g">
                                        <GameDisplay :game="g"></GameDisplay>
                                    </button>
                                </li>
                            </ul>
                        </div>
                    </th>
                    <th colspan="2">
                        <div class="dropdown">
                            Status
                            <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown"
                                data-bs-auto-close="outside">
                                <i :class="[
                                    (progressionFilter.isActive.value || completionFilter.isActive.value) ?
                                        'bi-funnel-fill'
                                        : 'bi-funnel'
                                ]"></i>
                            </button>
                            <ul class="dropdown-menu">
                                <li v-for="status in progressionStatus">
                                    <button class="dropdown-item" :class="progressionFilter.classes(status)"
                                        @click="progressionFilter.toggle(status)">
                                        {{ status.label }}
                                    </button>
                                </li>
                                <li>
                                    <hr class="dropdown-divider">
                                </li>
                                <li v-for="status in completionStatus">
                                    <button class="dropdown-item" :class="completionFilter.classes(status)"
                                        @click="completionFilter.toggle(status)">
                                        {{ status.label }}
                                    </button>
                                </li>
                            </ul>
                        </div>
                    </th>
                    <th :colspan="showLastActivity ? 3 : 2">
                        <div class="dropdown">
                            <span class="sorter" @click="setSort(sortByActivity, true)">
                                Last Activity (Days)
                                <i v-if="activeSort[0] === sortByActivity" class="me-1"
                                    :class="{ 'bi-sort-numeric-down': !activeSort[1], 'bi-sort-numeric-up': activeSort[1] }"></i>
                            </span>
                            <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown" data-bs-auto-close="outside">
                                <i class="bi-gear"></i>
                            </button>
                            <form class="dropdown-menu dropdown-menu-end p-4">
                                <div>
                                    <div class="form-check">
                                        <input type="checkbox" class="form-check-input" id="showLastActivityCheck" v-model="showLastActivity">
                                        <label class="form-check-label" for="showLastActivityCheck">
                                            Show last activity if before last checked
                                        </label>
                                    </div>
                                </div>
                            </form>
                        </div>
                    </th>
                    <th>Checks</th>
                    <th>
                        <button class="btn btn-sm btn-outline-light" @click="setAllExpanded(!allExpanded)">Hints <i
                                :class="{ 'bi-arrows-angle-expand': !allExpanded, 'bi-arrows-angle-contract': allExpanded }"></i></button>
                    </th>
                </tr>
            </thead>
            <tbody>
                <tr v-if="sortedAndFilteredGames.length === 0">
                    <td :colspan="showLastActivity ? 13 : 12" class="text-center text-muted">
                        No slots match the selected filters.
                    </td>
                </tr>
                <template v-for="game in sortedAndFilteredGames">
                    <tr>
                        <td>
                            <a :href="`https://archipelago.gg/tracker/${trackerData.tracker_id}/0/${game.position}`"
                                target="_blank" class="text-reset mw-underline-hover">{{
                                    game.name }}</a>
                        </td>
                        <td>
                            <span v-if="game.discord_username && isGameCompleted(game)" class="text-danger">Never</span>
                            <DropdownSelector v-else-if="game.discord_username" :options="pingPreference"
                                :value="pingPreference.byId[game.discord_ping]" :disabled="loading"
                                @selected="s => setPing(game, s)"></DropdownSelector>
                        </td>
                        <td>
                            <DropdownSelector :options="availabilityStatus"
                                :value="availabilityStatus.byId[game.availability_status]" :disabled="loading"
                                @selected="s => setGameAvailabilityStatus(game, s)">
                            </DropdownSelector>
                        </td>
                        <td>
                            <template v-if="currentUser">
                                <button v-if="!game.discord_username" class="btn btn-sm btn-outline-secondary"
                                    :disabled="loading" @click="claimGame(game)">Claim</button>

                                <template v-if="game.discord_username && !isEqual(getClaimingUser(game), currentUser)">
                                    <button class="btn btn-sm btn-outline-secondary" :disabled="loading"
                                        data-bs-toggle="dropdown">Claim</button>
                                    <div class="dropdown-menu text-warning p-3">
                                        <span class="text-warning me-2 d-inline-block align-middle">Another user has claimed
                                            this
                                            slot.</span>
                                        <button class="btn btn-sm btn-warning" @click="claimGame(game)">Claim
                                            anyway</button>
                                    </div>
                                </template>

                                <button v-if="isEqual(getClaimingUser(game), currentUser)"
                                    class="btn btn-sm btn-outline-warning" :disabled="loading"
                                    @click="unclaimGame(game)">Disclaim</button>
                            </template>
                        </td>
                        <td>
                            <UsernameDisplay :user="getClaimingUser(game)"></UsernameDisplay>
                        </td>
                        <td>
                            <GameDisplay :game="game.game"></GameDisplay>
                        </td>
                        <td>
                            <DropdownSelector v-if="!includes(['done', 'released'], game.completion_status)"
                                :options="progressionStatus" :value="progressionStatus.byId[game.progression_status]"
                                :disabled="loading" @selected="s => setGameProgressionStatus(game, s)"></DropdownSelector>
                        </td>
                        <td>
                            <DropdownSelector :options="completionStatus"
                                :value="completionStatus.byId[game.completion_status]" :disabled="loading"
                                @selected="s => setGameCompletionStatus(game, s)">
                            </DropdownSelector>
                        </td>
                        <td class="text-end">
                            <span :class="[lastCheckedClass(game)]" :title="displayDateTime(getLastCheckedOrLastActivity(game))">{{
                                displayLastCheckedOrLastActivity(game) }}
                            </span>
                        </td>
                        <td v-if="showLastActivity">
                            <span v-if="game.last_activity < game.last_checked" :title="displayDateTime(game.last_activity)">
                                ({{ displayLastActivity(game) }})
                            </span>
                        </td>
                        <td class="text-start p-0">
                            <button class=" btn btn-sm btn-outline-secondary"
                                :class="{ invisible: game.progression_status !== 'bk' || isGameCompleted(game) }" :disabled="loading"
                                @click="updateLastChecked(game)">Still BK</button>
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
                        <td :colspan="showLastActivity ? 13 : 12" class="container-fluid">
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
                                            @click="copyHints(displayHintsByGame(game.id))"><i class="bi-copy"></i> Copy
                                            all</button>
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
                                                            title="Copy to clipboard"><i class="bi-copy"></i></a>
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
                                        <div v-for="status in progressionStatus" class="progress-bar"
                                            :class="[`bg-${status.color}`]"
                                            :style="{ width: `${percent(statChecksByStatusProgression(status.id), statTotalChecks)}%` }">
                                        </div>
                                        <div class="progress-bar bg-success"
                                            :style="{ width: `${percent(sumBy(statGames, 'checks_done'), statTotalChecks)}%` }">
                                        </div>
                                    </div>
                                </td>
                            </tr>
                            <tr>
                                <th class="text-end shrink-column">Completion</th>
                                <td class="align-middle">
                                    <div class="progress">
                                        <div class="progress-bar bg-danger"
                                            :style="{ width: `${percent(filter(statGames, g => g.completion_status === 'incomplete' && g.progression_status === 'bk').length, statGames.length)}%` }">
                                        </div>
                                        <div v-for="status in completionStatus" class="progress-bar"
                                            :class="[`bg-${status.color}`]"
                                            :style="{ width: `${percent(statGamesByCompletionStatus[status.id]?.length || 0, statGames.length)}%` }">
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
                <TrackerSummary :tracker-data="trackerData" summarize-by="owner"></TrackerSummary>
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

.sorter {
    cursor: pointer;
}

.mw-underline-hover {
    text-decoration: none;
}

.mw-underline-hover:hover {
    text-decoration: underline;
}

.dropdown-menu {
    max-height: 50vh;
    overflow-y: auto;
}
</style>