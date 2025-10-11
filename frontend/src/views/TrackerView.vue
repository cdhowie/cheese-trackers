<script setup>
// This view could use some refactoring to break stuff out into components.

import { computed, onUnmounted, ref, useTemplateRef, watch } from 'vue';
import { debouncedRef, refDebounced } from '@vueuse/core';
import { groupBy, keyBy, orderBy, sumBy, uniq, map, filter, reduce, join, includes, uniqBy, fromPairs, every, omit, findIndex, pick, some } from 'lodash-es';
import moment from 'moment';

import { settings, currentUser } from '@/settings';
import { now } from '@/time';
import { getTracker as apiGetTracker, updateGame as apiUpdateGame, updateTracker as apiUpdateTracker, updateHint as apiUpdateHint, setDashboardOverrideStatus as apiSetDashboardOverrideStatus } from '@/api';
import { progressionStatus, completionStatus, availabilityStatus, pingPreference, pingPolicy, hintClassification, unifiedGameStatus, getClaimingUserForGame as getClaimingUser, dashboardOverrideVisibilities, usersEqual } from '@/types';
import { percent, synchronize } from '@/util';
import { copy as clipboardCopy } from '@/clipboard';
import { currentError as globalError } from '@/error-modal';

import TrackerSummary from '@/components/TrackerSummary.vue';
import ChecksBar from '@/components/ChecksBar.vue';
import UsernameDisplay from '@/components/UsernameDisplay.vue';
import GameDisplay from '@/components/GameDisplay.vue';
import DropdownSelector from '@/components/DropdownSelector.vue';
import Repeat from '@/components/Repeat.vue';
import HintDisplay from '@/components/HintDisplay.vue';
import CancelableEdit from '@/components/CancelableEdit.vue';
import RoomPortButton from '@/components/RoomPortButton.vue';
import OverwriteClaimButton from '@/components/OverwriteClaimButton.vue';
import TrackerDescription from '@/components/TrackerDescription.vue';

import TrackerTable from '@/components/TrackerTable.vue';
import TrackerTableHeader from '@/components/TrackerTableHeader.vue';
import TrackerTableSlot from '@/components/TrackerTableSlot.vue';

import TrackerContainer from '@/components/TrackerContainer.vue';
import TrackerContainerHeader from '@/components/TrackerContainerHeader.vue';
import TrackerContainerSlot from '@/components/TrackerContainerSlot.vue';

const layouts = {
    table: {
        primary: TrackerTable,
        header: TrackerTableHeader,
        slot: TrackerTableSlot,
    },
    container: {
        primary: TrackerContainer,
        header: TrackerContainerHeader,
        slot: TrackerContainerSlot,
    },
};

const props = defineProps(['aptrackerid']);

const loading = ref(false);
const error = ref(undefined);
const showTools = ref(false);
const trackerData = ref(undefined);
const hintsByFinder = ref(undefined);
const hintsByReceiver = ref(undefined);
const gameById = ref(undefined);

const updateTrackerErrorCount = ref(0);

const roomHost = computed(() => {
    if (trackerData.value?.room_host) {
        return trackerData.value?.room_host;
    }

    if (trackerData.value?.room_link?.length) {
        try {
            const url = new URL(trackerData.value.room_link);
            return url.hostname;
        } catch (e) {}
    }
});

const layout = computed(() =>
    trackerData.value?.games?.length >= 1000 ?
        layouts.container :
        layouts.table
);

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

const canEditTrackerSettings = computed(() =>
    currentUserIsTrackerOwner.value || !trackerData.value.lock_settings
);

const canClaimGames = computed(() =>
    currentUser.value && (
        !trackerData.value?.require_authentication_to_claim ||
        currentUser.value.id !== undefined
    )
);

function claimTracker() {
    updateTracker({
        owner_ct_user_id: currentUser.value.id,
        // This will be ignored by the server, but is used to update our local
        // state after the request.
        owner_discord_username: currentUser.value.discordUsername,
        // Default settings to locked.
        lock_settings: true,
    });
}

function disclaimTracker() {
    updateTracker({
        owner_ct_user_id: undefined,
        owner_discord_username: undefined,
        lock_settings: false,
        require_authentication_to_claim: false,
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
        filter(players.value, p => !usersEqual(p, currentUser.value)) :
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
const showChecksAsPercent = ref(false);

const lastCheckedThresholds = computed(() => {
    const td = trackerData.value;
    if (!td) {
        return [];
    }

    return [
        { days: td.inactivity_threshold_red_hours / 24, color: 'danger' },
        { days: td.inactivity_threshold_yellow_hours / 24, color: 'warning' },
        { color: 'success' },
    ];
});

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
    return completionStatus.byId[game.completion_status]?.complete;
}

function canEditGame(game) {
    return !settings.value.protectOtherSlots ||
        game.availability_status === 'public' ||
        currentUserIsTrackerOwner.value ||
        usersEqual(getClaimingUser(game), currentUser.value);
}

function effectivePingPreference(game) {
    if (isGameCompleted(game)) {
        return pingPreference.byId.never;
    }

    if (trackerData.value.global_ping_policy) {
        return pingPolicy.byId[trackerData.value.global_ping_policy];
    }

    return { ...pingPreference.byId[game.discord_ping], editable: true };
}

function lastCheckedClass(game) {
    if (isGameCompleted(game)) {
        return 'text-success';
    }

    const sinceDays = gameDaysSinceLastCheckedOrLastActivity(game);

    if (sinceDays === undefined) {
        return 'text-danger';
    }

    for (const t of lastCheckedThresholds.value) {
        if (t.days === undefined || sinceDays >= t.days) {
            return `text-${t.color}`;
        }
    }
}

function displayLastCheckedOrLastActivity(game) {
    const days = gameDaysSinceLastCheckedOrLastActivity(game);

    return days === undefined ? 'Never' : `${days.toFixed(1)}d`;
}

function displayLastActivity(game) {
    const d = game.last_activity;

    return d === undefined ? 'Never' : `${Math.max(0, dateToDays(d)).toFixed(1)}d`;
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

const statGamesByUnifiedStatus = computed(() =>
    groupBy(statGames.value, g => unifiedGameStatus.forGame(g).id)
);

const statGamesByProgressionStatus = computed(() =>
    groupBy(statGames.value, 'progression_status')
);

const SUMMARY_PROGRESSION_STATUS = map(
    ['unknown', 'bk', 'soft_bk', 'unblocked', 'go'],
    (k) => {
        const s = progressionStatus.byId[k];
        if (k === 'go') {
            return { ...s, color: 'light' };
        }
        return s;
    }
);

const SUMMARY_UNIFIED_STATUS = map(
    ['bk', 'soft_bk', 'incomplete', 'all_checks', 'goal', 'done'],
    (k) => unifiedGameStatus.byId[k]
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

const freeFilterActive = ref(false);
const freeFilterInput = useTemplateRef('free-filter-input');

watch(freeFilterActive, (isActive) => {
    if (isActive) {
        setTimeout(() => {
            freeFilterInput.value?.focus();
        });
    }
});

const freeFilterText = ref('');
const freeFilterTextDebounced = refDebounced(freeFilterText, 500);
const freeFilterTextLowercase = computed(() =>
    freeFilterActive.value
        ? freeFilterTextDebounced.value.toLowerCase()
        : ''
);

const filteredGames = computed(() =>
    filter(trackerData.value?.games, g => {
        const user = getClaimingUser(g);

        return every(
            [progressionFilter, completionFilter, availabilityFilter], f => f.showGame(g)
        ) && (
            playerFilter.value === PLAYER_FILTER_ALL ? true :
                playerFilter.value === PLAYER_FILTER_UNOWNED ? !user :
                    usersEqual(playerFilter.value, user)
        ) && (
            gameFilter.value === undefined ||
            gameFilter.value === g.game
        ) && (
            freeFilterTextLowercase.value === '' || some(
                [
                    g.name,
                    g.effective_discord_username,
                    g.game,
                    g.notes,
                ],
                (text) => ('string' === typeof text) && text.toLowerCase().includes(freeFilterTextLowercase.value),
            )
        )
    })
);

function chainCompare(...fns) {
    return (a, b) => {
        for (const fn of fns) {
            const r = fn(a, b);

            if (!isNaN(r) && r !== 0) {
                return r;
            }
        }

        return 0;
    };
}

function compareByIteratee(f) {
    return (a, b) => {
        const x = f(a);
        const y = f(b);

        if (x < y) {
            return -1;
        }

        if (x > y) {
            return 1;
        }

        return 0;
    };
}

function reverseCompare(f) {
    return (a, b) => -f(a, b);
}

const sortByName = (() => {
    const collator = new Intl.Collator(undefined, { numeric: true });

    return (a, b) => collator.compare(a.name, b.name);
})();

const sortByGame = compareByIteratee(g => g.game.toLowerCase());

const sortByActivity = compareByIteratee(g => {
    const days = gameDaysSinceLastCheckedOrLastActivity(g);
    return days === undefined ? Number.POSITIVE_INFINITY : days;
});

const sortByOwner = compareByIteratee(g => (g.effective_discord_username || '').toLowerCase());

const sortByChecks = compareByIteratee(g => g.checks_done / g.checks_total);

const sortByHints = compareByIteratee(countUnfoundReceivedHints);

const activeSort = ref([sortByName, false]);

function setSort(sorter, defOrder) {
    if (activeSort.value[0] === sorter) {
        activeSort.value[1] = !activeSort.value[1];
    } else {
        activeSort.value = [sorter, defOrder];
    }
}

const SORT_MODES = {
    normal: () => 0,
    selftop: (g) => usersEqual(getClaimingUser(g), currentUser.value) ? 0 : 1,
};

const sortedAndFilteredGames = computed(() =>
    [...filteredGames.value].sort(
        chainCompare(
            compareByIteratee(SORT_MODES[settings.value.sortMode]),

            activeSort.value[1] ?
                reverseCompare(activeSort.value[0]) :
                activeSort.value[0],

            // Fall back on sorting by slot name.
            sortByName,
        )
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
const showFoundHints = ref(false);

const hintsByGame = computed(() => {
    return sentHints.value ? hintsByReceiver.value : hintsByFinder.value;
});

function hintStatus(hint) {
    function isDone(g) {
        return g.checks_done === g.checks_total && (
            g.completion_status === 'done' ||
            g.completion_status === 'released'
        );
    }

    return hint.found ? 'found' :
        (
            hint.receiver_game_id !== undefined &&
            isDone(gameById.value[hint.receiver_game_id])
        ) ? 'useless' :
            'notfound';
}

function displayHintsByGame(id) {
    return (hintsByGame.value?.[id] || []).filter(h =>
        showFoundHints.value || (
            hintStatus(h) === 'notfound' &&
            h.classification !== 'trash'
        )
    );
}

const HINT_STATUS_ORDER = {
    notfound: 0,
    useless: 1,
    found: 2,
}

function sortedDisplayHintsByGame(id) {
    return orderBy(
        displayHintsByGame(id),
        [
            h => HINT_STATUS_ORDER[hintStatus(h)],
            h => findIndex(hintClassification, c => c.id === h.classification),
            'id',
        ]
    );
}

function countUnfoundReceivedHints(game) {
    return (hintsByFinder.value[game.id] || [])
        .filter(h =>
            h.receiver_game_id !== game.id &&
            h.classification !== 'trash' &&
            hintStatus(h) === 'notfound'
        )
        .length;
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
        handleTrackerResponse(data);
    } catch (e) {
        error.value = e;
    } finally {
        loading.value = false;
    }
}

function handleTrackerResponse(data) {
    data.games.forEach(patchGame);
    trackerData.value = data;

    hintsByFinder.value = groupBy(trackerData.value.hints, 'finder_game_id');
    hintsByReceiver.value = groupBy(filter(trackerData.value.hints, h => h.receiver_game_id !== undefined), 'receiver_game_id');
    gameById.value = keyBy(trackerData.value.games, 'id');
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
    }, true);
}

function unclaimGame(game) {
    updateGame(game, g => {
        delete g.claimed_by_ct_user_id;
        delete g.discord_username;

        if (g.availability_status === 'claimed') {
            g.availability_status = 'open';
        }

        g.discord_ping = 'never';
    }, true);
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
        if (progressionStatus.byId[status].isBk) {
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

function setHintClassification(hint, classification) {
    setTimeout(() => updateHint(hint, h => {
        h.classification = classification.id || classification;
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

function hintToString(hint) {
    const receiver = gameById.value[hint.receiver_game_id]?.name || (
        hint.item_link_name !== '' ? `[LINK] ${hint.item_link_name}` : '(Item link)'
    );
    const finder = gameById.value[hint.finder_game_id].name;
    const entrance = hint.entrance === 'Vanilla' ? '' : ` (${hint.entrance})`;
    return `${receiver}'s ${hint.item} is at ${finder}'s ${hint.location}${entrance}`;
}

function copyHints(hints) {
    clipboardCopy(join(map(hints, hintToString), '\n'))
}

function hintToStringWithPing(hint) {
    const otherSlot = gameById.value[sentHints.value ? hint.finder_game_id : hint.receiver_game_id];

    return `${hintToString(hint)} @${otherSlot.effective_discord_username} `;
}

async function updateObject(data, updater, mutator, patcher) {
    if (loading.value) {
        return;
    }

    loading.value = true;

    const newData = { ...data };
    if (mutator) {
        mutator(newData);
    }

    return updater(newData)
        .then(
            ({ status, data }) => status >= 200 && status < 300 ? data : undefined,
            e => {
                console.error(`Failed to update: ${e}`);
                loading.value = false;
                throw e;
            }
        )
        .then(saved => {
            loading.value = false;
            if (saved) {
                synchronize(data, saved);
            }
            if (patcher) {
                patcher(data);
            }
            return saved;
        });
}

async function updateTracker(data) {
    try {
        const r = await updateObject(
            Object.assign(
                omit(trackerData.value, 'games', 'hints'),
                data
            ),
            apiUpdateTracker
        );

        handleTrackerResponse(r);
    } catch (e) {
        updateTrackerErrorCount.value += 1;
        console.log(e);
        throw e;
    }
}

async function updateGame(game, mutator, isClaimChange) {
    const priorOwner = isClaimChange
        ? pick(game, 'claimed_by_ct_user_id', 'discord_username')
        : undefined;

    try {
        return await updateObject(
            game,
            g => apiUpdateGame(props.aptrackerid, g, priorOwner),
            mutator,
            patchGame
        );
    } catch (e) {
        if (e.status === 412) {
            globalError.value = `The owner information for the slot "${game.name}" is out of date, so you cannot modify the claim.  Please refresh the tracker.`;
        } else {
            throw e;
        }
    }
}

async function updateHint(hint, mutator) {
    return updateObject(
        hint,
        h => apiUpdateHint(props.aptrackerid, h),
        mutator
    );
}

function setDashboardOverrideStatus(status) {
    setTimeout(() => {
        updateObject(
            {},
            () => apiSetDashboardOverrideStatus(props.aptrackerid, status),
            undefined,
            (r) => {
                if (r) {
                    trackerData.value.dashboard_override_visibility = r.visibility;
                }
            }
        );
    });
}

loadTracker();
</script>

<template>
    <div v-if="loading && !trackerData" class="placeholder-wave">
        <div class="text-center">
            <h2 class="placeholder me-1" style="width: 10em"></h2>
            <h2 class="placeholder me-1" style="width: 2em"></h2>
            <h2 class="placeholder" style="width: 4em"></h2>
        </div>
        <table class="table">
            <thead>
                <tr>
                    <Repeat times="9">
                        <th><div class="placeholder w-100"></div></th>
                    </Repeat>
                </tr>
            </thead>
            <tbody>
                <Repeat times="10">
                    <tr>
                        <Repeat times="9">
                            <td><div class="placeholder bg-secondary w-100"></div></td>
                        </Repeat>
                    </tr>
                </Repeat>
            </tbody>
        </table>
    </div>
    <div v-if="error && !trackerData" class="text-center text-danger">
        Failed to load tracker data ({{ error.message }})
    </div>
    <template v-if="trackerData">
        <div v-if="!currentUser" class="alert alert-info text-center">
            You will be unable to claim slots until you either sign in with Discord or set your Discord username in the
            <router-link to="/settings">settings</router-link>.
        </div>
        <div :class="(showTools || (trackerData?.description || '').length) ? 'mb-3' : 'mb-4'">
            <h2 class="text-center">
                <span :class="{ 'text-muted': !trackerData.title, 'fst-italic': !trackerData.title }">{{
                    trackerData.title.length ?
                    trackerData.title : 'Untitled tracker' }}
                </span>
                <template v-if="trackerOwner">
                    by <UsernameDisplay :user="trackerOwner"></UsernameDisplay>
                </template> <button
                    class="btn btn-sm btn-outline-light"
                    :class="{ active: showTools }"
                    @click="showTools = !showTools"
                >
                    <i :class="showTools ? 'bi-gear-fill' : 'bi-gear'"/>
                </button> <DropdownSelector
                    v-if="currentUser?.id !== undefined"
                    :options="dashboardOverrideVisibilities"
                    :value="dashboardOverrideVisibilities.byId[trackerData.dashboard_override_visibility]"
                    :disabled="loading"
                    :icons="true"
                    @selected="(s) => setDashboardOverrideStatus(s.id)"
                /> <div class="input-group input-group-sm d-inline-flex align-bottom w-auto">
                    <button
                        class="btn btn-outline-light"
                        :class="{ active: freeFilterActive }"
                        @click="freeFilterActive = !freeFilterActive"
                    >
                        <i class="bi-search"/>
                    </button>
                    <input
                        v-if="freeFilterActive"
                        ref="free-filter-input"
                        class="form-control"
                        placeholder="Find slot"
                        v-model="freeFilterText"
                    >
                </div>
            </h2>
            <div
                v-if="trackerData?.room_link?.length"
                class="text-center"
            >
                <a
                    :href="trackerData.room_link"
                    target="_blank"
                    alt="Room"
                    class="badge text-bg-info"
                >
                    <i class="bi-door-open-fill"></i>
                </a> <RoomPortButton
                    :host="roomHost"
                    :port="trackerData?.last_port"
                />
            </div>
        </div>
        <TrackerDescription
            v-if="!showTools && (trackerData?.description || '').length && trackerOwner"
            class="container bg-dark-subtle pt-3 pb-3 mb-4 rounded"
            :source="trackerData.description"
        />
        <form class="container bg-dark-subtle pt-3 mb-4 rounded" v-if="showTools">
            <div class="row">
                <div class="col-12 col-xxl-6 mb-3">
                    <div class="row">
                        <label class="col-form-label col-3">Organizer</label>
                        <div class="col-9">
                            <div class="input-group">
                                <button v-if="currentUser?.id !== undefined && !trackerOwner"
                                    type="button"
                                    class="btn btn-outline-secondary"
                                    :disabled="loading"
                                    @click="claimTracker">Claim</button>
                                <button v-if="currentUserIsTrackerOwner"
                                    type="button"
                                    class="btn btn-outline-warning"
                                    :disabled="loading"
                                    @click="disclaimTracker">Disclaim</button>
                                <input type="text" disabled="disabled"
                                    class="form-control"
                                    placeholder="(Unclaimed)"
                                    :value="trackerOwner?.discordUsername">
                            </div>
                        </div>
                    </div>
                </div>
                <div class="col-12 col-xxl-6 mb-3">
                    <div class="row">
                        <label class="col-form-label col-3" for="trackerLockSettingsCheck">Lock settings</label>
                        <div class="col-9">
                            <div class="form-control-plaintext form-check form-switch">
                                <input
                                    :disabled="loading || !currentUserIsTrackerOwner"
                                    class="form-check-input"
                                    type="checkbox"
                                    role="switch"
                                    id="trackerLockSettingsCheck"
                                    :checked="trackerData.lock_settings"
                                    @change="
                                        updateTracker({ lock_settings: !trackerData.lock_settings }).catch(() => {
                                            $event.target.checked = trackerData.lock_settings;
                                        });
                                    "
                                >
                            </div>
                        </div>
                    </div>
                </div>
                <div class="col-12 col-xxl-6 mb-3">
                    <div class="row">
                        <label class="col-form-label col-3" for="trackerTitleEdit">Title</label>
                        <div class="col-9">
                            <CancelableEdit
                                :modelValue="trackerData?.title || ''"
                                :reset="updateTrackerErrorCount"
                                @update:modelValue="(title) => updateTracker({ title })"
                                v-slot="props"
                            >
                                <input
                                    type="text"
                                    id="trackerTitleEdit"
                                    :disabled="loading || !canEditTrackerSettings"
                                    class="form-control"
                                    :value="props.value"
                                    @input="(e) => props.edited(e.target.value)"
                                    placeholder="Title"
                                    @blur="props.save()"
                                    @keyup.enter.prevent="props.save()"
                                    @keyup.esc="props.cancel()"
                                >
                            </CancelableEdit>
                        </div>
                    </div>
                </div>
                <div class="col-12 col-xxl-6 mb-3">
                    <div class="row">
                        <label class="col-form-label col-3" for="trackerRoomLinkEdit">Room link</label>
                        <div class="col-9">
                            <CancelableEdit
                                :modelValue="trackerData?.room_link"
                                :reset="updateTrackerErrorCount"
                                @update:modelValue="(room_link) => updateTracker({ room_link })"
                                v-slot="props"
                            >
                                <input
                                    type="text"
                                    id="trackerRoomLinkEdit"
                                    :disabled="loading || !canEditTrackerSettings"
                                    class="form-control"
                                    :value="props.value"
                                    @input="(e) => props.edited(e.target.value)"
                                    placeholder="Room link"
                                    @blur="props.save()"
                                    @keyup.enter.prevent="props.save()"
                                    @keyup.esc="props.cancel()"
                                >
                            </CancelableEdit>
                        </div>
                    </div>
                </div>
                <div class="col-12 col-xxl-6 mb-3">
                    <div class="row">
                        <label class="col-form-label col-3">Inactivity thresholds</label>
                        <div class="col-9">
                            <div class="row">
                                <div class="col-6">
                                    <CancelableEdit
                                        :modelValue="trackerData?.inactivity_threshold_yellow_hours"
                                        :reset="updateTrackerErrorCount"
                                        @update:modelValue="(h) => updateTracker({ inactivity_threshold_yellow_hours: +h })"
                                        v-slot="props"
                                    >
                                        <div class="input-group">
                                            <input
                                                type="number"
                                                min="0"
                                                :disabled="loading || !canEditTrackerSettings"
                                                class="form-control"
                                                :value="props.value"
                                                @input="(e) => props.edited(e.target.value)"
                                                @change="(e) => { props.edited(e.target.value); e.target.focus(); }"
                                                @blur="props.save()"
                                                @keyup.enter.prevent="props.save()"
                                                @keyup.esc="props.cancel()"
                                            >
                                            <span class="input-group-text text-warning">hours</span>
                                        </div>
                                    </CancelableEdit>
                                </div>
                                <div class="col-6">
                                    <CancelableEdit
                                        :modelValue="trackerData?.inactivity_threshold_red_hours"
                                        :reset="updateTrackerErrorCount"
                                        @update:modelValue="(h) => updateTracker({ inactivity_threshold_red_hours: +h })"
                                        v-slot="props"
                                    >
                                        <div class="input-group">
                                            <input
                                                type="number"
                                                min="0"
                                                :disabled="loading || !canEditTrackerSettings"
                                                class="form-control"
                                                :value="props.value"
                                                @input="(e) => props.edited(e.target.value)"
                                                @change="(e) => { props.edited(e.target.value); e.target.focus(); }"
                                                @blur="props.save()"
                                                @keyup.enter.prevent="props.save()"
                                                @keyup.esc="props.cancel()"
                                            >
                                            <span class="input-group-text text-danger">hours</span>
                                        </div>
                                    </CancelableEdit>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="col-12 col-xxl-6 mb-3">
                    <div class="row">
                        <label class="col-form-label col-3">Ping policy</label>
                        <div class="col-9">
                            <div class="btn-group form-control border-0 p-0">
                                <template v-for="pref of pingPolicy">
                                    <button
                                        type="button"
                                        class="btn"
                                        :disabled="loading || !canEditTrackerSettings"
                                        :class="{
                                            [`btn-outline-${pref.color}`]: trackerData.global_ping_policy !== pref.id,
                                            [`btn-${pref.color}`]: trackerData.global_ping_policy === pref.id,
                                        }"
                                        @click.prevent="updateTracker({ global_ping_policy: pref.id })"
                                    >{{ pref.label }}</button>
                                </template>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="col-12 col-xxl-6 mb-3" v-if="trackerData.lock_settings">
                    <div class="row">
                        <label class="col-form-label col-6" for="trackerRequireAuthCheck">Require authentication to claim</label>
                        <div class="col-6">
                            <div class="form-control-plaintext form-check form-switch">
                                <input
                                    :disabled="loading || !currentUserIsTrackerOwner"
                                    class="form-check-input"
                                    type="checkbox"
                                    role="switch"
                                    id="trackerRequireAuthCheck"
                                    :checked="trackerData.require_authentication_to_claim"
                                    @change="
                                        updateTracker({ require_authentication_to_claim: !trackerData.require_authentication_to_claim }).catch(() => {
                                            $event.target.checked = trackerData.require_authentication_to_claim;
                                        });
                                    "
                                >
                            </div>
                        </div>
                    </div>
                </div>
                <div class="col-12 mb-3" v-if="currentUserIsTrackerOwner">
                    <CancelableEdit
                        :modelValue="trackerData?.description || ''"
                        :reset="updateTrackerErrorCount"
                        @update:modelValue="(d) => updateTracker({ description: d})"
                        v-slot="props"
                    >
                        <label class="form-label" for="trackerDescriptionEdit">Description</label>
                        <textarea
                            id="trackerDescriptionEdit"
                            :disabled="loading"
                            class="form-control mb-2"
                            rows="10"
                            :value="props.value"
                            @input="e => props.edited(e.target.value)"
                            placeholder="Description"
                            @blur="props.save()"
                            @keyup.esc="props.cancel()"
                        />
                        <div class="alert alert-warning">
                            The description can be viewed by anyone who has a
                            link to the tracker, even if the link is a read-only
                            link. <b>Do not include information in the
                            description if it is only intended for participants
                            and not observers.</b> For example, putting the room
                            link in the description would allow someone who has
                            a read-only tracker link to connect to the
                            multiworld server, and even obtain a read-write
                            tracker link via the multiworld tracker link
                            available on the room page.
                        </div>
                        <label class="form-label">Preview</label>
                        <TrackerDescription
                            class="form-control pt-3 pb-3"
                            :source="props.value"
                        />
                    </CancelableEdit>
                </div>
                <div class="col-12 mb-3" v-else-if="trackerOwner && (trackerData?.description || '').length">
                    <label class="form-label">Description</label>
                    <TrackerDescription
                        class="form-control pt-3 pb-3"
                        :source="trackerData.description"
                    />
                </div>
            </div>
        </form>
        <button class="btn btn-primary refresh-button" @click="loadTracker()" :disabled="loading">Refresh</button>
        <component :is="layout.primary" :items="sortedAndFilteredGames">
            <template #head>
                <component :is="layout.header" :show-last-activity="showLastActivity">
                    <template #name>
                        <span @click="setSort(sortByName, false)" class="sorter">
                            Name
                            <i v-if="activeSort[0] === sortByName"
                                :class="{ 'bi-sort-alpha-down': !activeSort[1], 'bi-sort-alpha-up': activeSort[1] }"></i>
                        </span>
                    </template>
                    <template #ping>Ping</template>
                    <template #availability>
                        Availability
                        <button class="btn btn-sm btn-outline-light" data-bs-toggle="dropdown" data-bs-auto-close="outside">
                            <i :class="[availabilityFilter.isActive.value ? 'bi-funnel-fill' : 'bi-funnel']"></i>
                        </button>
                        <ul class="dropdown-menu">
                            <li v-for="status in availabilityStatus">
                                <button class="dropdown-item" :class="availabilityFilter.classes(status)"
                                    @click="availabilityFilter.toggle(status)">
                                    <i :class="`bi-${status.icon}`"></i> {{ status.label }}
                                </button>
                            </li>
                        </ul>
                    </template>
                    <template #owner>
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
                                            :class="{ active: usersEqual(playerFilter, currentUser) }"
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
                    </template>
                    <template #game>
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
                    </template>
                    <template #status>
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
                                        <i :class="`bi-${status.icon}`"></i> {{ status.label }}
                                    </button>
                                </li>
                                <li>
                                    <hr class="dropdown-divider">
                                </li>
                                <li v-for="status in completionStatus">
                                    <button class="dropdown-item" :class="completionFilter.classes(status)"
                                        @click="completionFilter.toggle(status)">
                                        <i :class="`bi-${status.icon}`"></i> {{ status.label }}
                                    </button>
                                </li>
                            </ul>
                        </div>
                    </template>
                    <template #lastactivity>
                        <div class="dropdown">
                            <span class="sorter" @click="setSort(sortByActivity, true)">
                                Last Activity
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
                    </template>
                    <template #checks>
                        <span class="sorter" @click="setSort(sortByChecks, false)">
                            Checks
                            <i
                                v-if="activeSort[0] === sortByChecks"
                                class="me-1"
                                :class="{
                                    'bi-sort-numeric-down': !activeSort[1],
                                    'bi-sort-numeric-up': activeSort[1],
                                }"
                            />
                        </span>
                        <button
                            class="btn btn-sm btn-outline-light"
                            :class="{ active: showChecksAsPercent }"
                            @click.prevent="showChecksAsPercent = !showChecksAsPercent"
                        >
                            <i class="bi-percent"/>
                        </button>
                    </template>
                    <template #hints>
                        <span class="sorter" @click="setSort(sortByHints, true)">
                            Hints
                            <i
                                v-if="activeSort[0] === sortByHints"
                                class="me-1"
                                :class="{
                                    'bi-sort-numeric-down': !activeSort[1],
                                    'bi-sort-numeric-up': activeSort[1],
                                }"
                            />
                        </span>
                        <button class="btn btn-sm btn-outline-light" @click="setAllExpanded(!allExpanded)">
                            <i :class="{ 'bi-arrows-angle-expand': !allExpanded, 'bi-arrows-angle-contract': allExpanded }"/>
                        </button>
                    </template>
                </component>
            </template>
            <template #empty>
                <component :is="layout.slot">
                    <template #banner>
                        <span class="text-muted">
                            No slots match the selected filters.
                        </span>
                    </template>
                    <template #activity v-if="showLastActivity"/>
                </component>
            </template>
            <template #game="{ game }">
                <component :is="layout.slot" :isMine="usersEqual(getClaimingUser(game), currentUser)">
                    <template #name>
                        <a
                            :href="`${trackerData.upstream_url}/0/${game.position}`"
                            target="_blank"
                            class="text-reset mw-underline-hover"
                        >{{ game.name }}</a>
                    </template>
                    <template #ping v-if="game.effective_discord_username">
                        <span
                            v-if="!effectivePingPreference(game).editable"
                            :class="`text-${effectivePingPreference(game).color}`"
                        >{{ effectivePingPreference(game).label }}</span>
                        <DropdownSelector
                            v-else
                            :options="pingPreference"
                            :value="pingPreference.byId[game.discord_ping]"
                            :disabled="loading"
                            :readonly="!canEditGame(game)"
                            @selected="s => setPing(game, s)"/>
                    </template>
                    <template #availability>
                        <DropdownSelector
                            :options="availabilityStatus"
                            :value="availabilityStatus.byId[game.availability_status]"
                            :disabled="loading"
                            :readonly="!canEditGame(game)"
                            :icons="settings.statusIcons"
                            @selected="s => setGameAvailabilityStatus(game, s)">
                        </DropdownSelector>
                    </template>
                    <template #claim>
                        <template v-if="currentUser">
                            <button v-if="canClaimGames && !game.effective_discord_username" class="btn btn-sm btn-outline-secondary"
                                :disabled="loading" @click="claimGame(game)">Claim</button>

                            <OverwriteClaimButton
                                v-else-if="
                                    canClaimGames &&
                                    game.effective_discord_username &&
                                    !usersEqual(getClaimingUser(game), currentUser) &&
                                    canEditGame(game)
                                "
                                :disabled="loading"
                                @claimed="claimGame(game)"
                            />

                            <button v-else-if="usersEqual(getClaimingUser(game), currentUser) && canEditGame(game)"
                                class="btn btn-sm btn-outline-warning" :disabled="loading"
                                @click="unclaimGame(game)">Disclaim</button>
                        </template>
                    </template>
                    <template #owner>
                        <UsernameDisplay :user="getClaimingUser(game)"></UsernameDisplay>
                    </template>
                    <template #game>
                        <GameDisplay :game="game.game"></GameDisplay>
                    </template>
                    <template #progression>
                        <DropdownSelector
                            v-if="!isGameCompleted(game)"
                            :options="progressionStatus"
                            :value="progressionStatus.byId[game.progression_status]"
                            :disabled="loading"
                            :readonly="!canEditGame(game)"
                            :icons="settings.statusIcons"
                            @selected="s => setGameProgressionStatus(game, s)"
                        ></DropdownSelector>
                    </template>
                    <template #completion>
                        <DropdownSelector
                            :options="completionStatus"
                            :value="completionStatus.byId[game.completion_status]"
                            :disabled="loading"
                            :readonly="!canEditGame(game)"
                            :icons="settings.statusIcons"
                            @selected="s => setGameCompletionStatus(game, s)">
                        </DropdownSelector>
                    </template>
                    <template #checked>
                        <span :class="[lastCheckedClass(game)]" :title="displayDateTime(getLastCheckedOrLastActivity(game))">{{
                            displayLastCheckedOrLastActivity(game) }}
                        </span>
                    </template>
                    <template #activity v-if="showLastActivity">
                        <span v-if="game.last_activity < game.last_checked" :title="displayDateTime(game.last_activity)">
                            ({{ displayLastActivity(game) }})
                        </span>
                    </template>
                    <template #stillbk>
                        <button
                            v-if="canEditGame(game)"
                            class="btn btn-sm btn-outline-secondary"
                            :class="{ invisible: !progressionStatus.byId[game.progression_status].isBk || isGameCompleted(game) }"
                            :disabled="loading"
                            @click="updateLastChecked(game)">Still BK</button>
                    </template>
                    <template #checks>
                        <ChecksBar
                            :done="game.checks_done"
                            :total="game.checks_total"
                            :show-percent="showChecksAsPercent && 'only'"
                        />
                    </template>
                    <template #hints>
                        <button class="btn btn-sm" :class="[hintsClass(game)]"
                            @click="gameExpanded[game.id] = !gameExpanded[game.id]">
                            {{ countUnfoundReceivedHints(game) }}<template v-if="game.notes !== ''">*</template> <i
                                :class="{ 'bi-arrows-angle-expand': !gameExpanded[game.id], 'bi-arrows-angle-contract': gameExpanded[game.id] }"></i>
                        </button>
                    </template>

                    <template #hintpane v-if="gameExpanded[game.id]">
                        <div class="row">
                            <div class="col-12" v-if="game.user_is_away">
                                <div class="alert alert-warning p-2">
                                    The owner of this slot is away.
                                </div>
                            </div>
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
                                        <table class="table table-responsive table-borderless table-sm hint-table">
                                            <tbody>
                                                <HintDisplay
                                                    v-for="hint in sortedDisplayHintsByGame(game.id)"
                                                    :hint="hint"
                                                    :status="hintStatus(hint)"
                                                    :show-status="showFoundHints"
                                                    :direction="sentHints ? 'sent' : 'received'"
                                                    :receiver-game="gameById[hint.receiver_game_id]"
                                                    :finder-game="gameById[hint.finder_game_id]"
                                                    :item-link-name="hint.item_link_name"
                                                    :global-ping-policy="trackerData.global_ping_policy && pingPolicy.byId[trackerData.global_ping_policy]"
                                                    :disabled="loading"
                                                    :readonly="!(
                                                        canEditGame(gameById[hint.finder_game_id]) || (
                                                            hint.receiver_game_id !== undefined &&
                                                            canEditGame(gameById[hint.receiver_game_id])
                                                        )
                                                    )"
                                                    @set-classification="s => setHintClassification(hint, s)"
                                                    @copy="clipboardCopy(hintToString(hint))"
                                                    @copy-ping="clipboardCopy(hintToStringWithPing(hint))"
                                                ></HintDisplay>
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            </div>
                            <div class="col-12 col-xl-6">
                                <div class="fw-bold">Notes</div>
                                <textarea class="form-control"
                                    rows="5" v-model="game.$newnotes" @blur="updateNotes(game)"
                                    :placeholder="canEditGame(game) ? 'Enter any notes about your game here.' : 'There are no notes for this slot.'"
                                    :disabled="loading || !canEditGame(game)"
                                    @keyup.esc="game.$newnotes = game.notes"></textarea>
                                <div class="text-muted">
                                    Saves automatically when you click off of the field. Press ESC to cancel any edits.
                                </div>
                            </div>
                        </div>
                    </template>
                </component>
            </template>
        </component>
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
                                        <div v-for="status in SUMMARY_PROGRESSION_STATUS" class="progress-bar"
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
                                        <div v-for="status in SUMMARY_UNIFIED_STATUS" class="progress-bar"
                                            :class="[`bg-${status.color}`]"
                                            :style="{ width: `${percent(statGamesByUnifiedStatus[status.id]?.length || 0, statGames.length)}%` }">
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
</template>

<style scoped>
.refresh-button {
    position: fixed;
    bottom: 1rem;
    right: 1.5rem;
    z-index: 1;
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

<style>
.hint-table button:not(.dropdown-item) {
    padding: 0.125em 0.25em;
}
</style>
