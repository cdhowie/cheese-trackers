import { keyBy, map } from "lodash-es";

function keyed(v) {
    v.byId = keyBy(v, 'id');
    return v;
}

export const progressionStatus = keyed([
    { id: 'unknown', label: 'Unknown', color: 'secondary', icon: 'question-lg', description: 'Progression not yet assessed' },
    { id: 'unblocked', label: 'Unblocked', color: 'light', icon: 'person-walking', description: 'Can make progress — items are available to collect' },
    { id: 'bk', isBk: true, label: 'BK', color: 'danger', icon: 'octagon-fill', description: 'Blocked — cannot progress without receiving a needed item from another player' },
    { id: 'soft_bk', isBk: true, label: 'Soft BK', color: 'warning', icon: 'octagon-half', description: 'Soft blocked — limited options remaining while waiting on a key item' },
    { id: 'go', label: 'Go mode', color: 'success', icon: 'flag', description: 'Go mode — has all needed items and is racing to the goal' },
]);

export const completionStatus = keyed([
    { id: 'incomplete', label: 'Incomplete', color: 'light', icon: 'square', description: 'Has not finished yet' },
    { id: 'all_checks', label: 'All checks', color: 'info', icon: 'check-square', description: 'Collected all location checks but has not reached the goal' },
    { id: 'goal', label: 'Goal', color: 'info', icon: 'flag', description: 'Reached the game goal but has not sent all items yet' },
    { id: 'done', label: 'Done', color: 'success', icon: 'flag-fill', complete: true, description: 'Fully completed — goal reached and all items sent' },
    { id: 'released', label: 'Forfeit', color: 'secondary', icon: 'escape', complete: true, description: 'Forfeited — all remaining items have been released to other players' },
]);

export const availabilityStatus = keyed([
    { id: 'unknown', label: 'Unknown', color: 'secondary', icon: 'question-lg', description: 'Availability not set' },
    { id: 'open', label: 'Open', color: 'success', icon: 'person', description: 'Open — available for anyone to claim' },
    { id: 'claimed', label: 'Claimed', color: 'light', icon: 'person-fill', description: 'Claimed by a specific player' },
    { id: 'public', label: 'Public', color: 'info', icon: 'people-fill', description: 'Public — anyone may send checks for this slot' },
]);

export const hintClassification = keyed([
    { id: 'unset', label: 'Unset', color: 'light', icon: 'envelope-exclamation-fill' },
    { id: 'critical', label: 'Critical', color: 'danger', icon: 'exclamation-triangle-fill' },
    { id: 'progression', label: 'Progression', color: 'warning', icon: 'person-walking' },
    { id: 'qol', label: 'Quality of life', color: 'primary', icon: 'person-raised-hand' },
    { id: 'trash', label: 'Trash', color: 'secondary', icon: 'trash-fill' },
    { id: 'unknown', label: 'Unknown', color: 'light', icon: 'question-lg' },
]);

export const pingPreference = keyed([
    { id: 'liberally', label: 'Liberally', color: 'success', pingWhen: 'liberally' },
    { id: 'sparingly', label: 'Sparingly', color: 'warning', pingWhen: 'sparingly' },
    { id: 'hints', label: 'Hints', color: 'warning', pingWhen: 'for hints' },
    { id: 'see_notes', label: 'See notes', color: 'info', pingWhen: 'after checking notes' },
    { id: 'never', label: 'Never', color: 'danger', pingWhen: 'never' },
]);

export const pingPolicy = keyed([
    { id: undefined, label: 'None', color: 'secondary' },
    { id: 'liberally', label: 'Liberally', color: 'success', pingWhen: 'liberally' },
    { id: 'sparingly', label: 'Sparingly', color: 'warning', pingWhen: 'sparingly' },
    { id: 'hints', label: 'Hints', color: 'warning', pingWhen: 'for hints' },
    { id: 'see_notes', label: 'Custom', color: 'info', pingWhen: "after checking the async's ping policy" },
    { id: 'never', label: 'Never', color: 'danger', pingWhen: 'never' },
]);

export const dashboardOverrideVisibilities = keyed([
    { id: undefined, label: 'Auto', color: 'light', icon: 'asterisk' },
    { id: true, label: 'Follow', color: 'light', icon: 'eye-fill' },
    { id: false, label: 'Ignore', color: 'light', icon: 'eye-slash-fill' },
]);

// A few views show a unified game status which is a combination of the
// completion status and progression status.  If the completion status is
// incomplete, then we use the progression status only if it's a BK type.
//
// Here we wrap up that logic and hybrid data type so it doesn't have to be
// reiterated in every component that needs it.

// We use this instead of deriving from isBk to control the order.
const INCOMPLETE_PROGRESSION_OVERRIDES = ['bk', 'soft_bk'];

export const unifiedGameStatus = keyed((() => [
    ...map(
        INCOMPLETE_PROGRESSION_OVERRIDES,
        k => progressionStatus.byId[k]
    ),
    ...completionStatus,
])());

unifiedGameStatus.forGame = (game) =>
    (
        game.completion_status === 'incomplete' &&
        progressionStatus.byId[game.progression_status].isBk
    ) ?
        progressionStatus.byId[game.progression_status] :
        completionStatus.byId[game.completion_status];

export function getClaimingUserForGame(game) {
    if (game.claimed_by_ct_user_id !== undefined) {
        return {
            id: game.claimed_by_ct_user_id,
            discordUsername: game.effective_discord_username,
            isAway: game.user_is_away,
        };
    }

    if (game.effective_discord_username?.length) {
        return {
            discordUsername: game.effective_discord_username,
            isAway: game.user_is_away,
        };
    }

    return undefined;
}

export const sortModes = keyed([
    { id: 'normal', label: 'Normal' },
    { id: 'selftop', label: 'Mine first' },
]);

export function usersEqual(a, b) {
    return !a ? !b : (b && a.id === b.id && a.discordUsername === b.discordUsername);
}
