import { keyBy } from "lodash-es";

function keyed(v) {
    v.byId = keyBy(v, 'id');
    return v;
}

export const progressionStatus = keyed([
    { id: 'unknown', label: 'Unknown', color: 'secondary' },
    { id: 'unblocked', label: 'Unblocked', color: 'light' },
    { id: 'bk', label: 'BK', color: 'danger' },
]);

export const completionStatus = keyed([
    { id: 'incomplete', label: 'Incomplete', color: 'light' },
    { id: 'all_checks', label: 'All checks', color: 'info' },
    { id: 'goal', label: 'Goal', color: 'info' },
    { id: 'done', label: 'Done', color: 'success' },
    { id: 'released', label: 'Released', color: 'secondary' },
]);

export const availabilityStatus = keyed([
    { id: 'unknown', label: 'Unknown', color: 'secondary' },
    { id: 'open', label: 'Open', color: 'success' },
    { id: 'claimed', label: 'Claimed', color: 'light' },
    { id: 'public', label: 'Public', color: 'info' },
]);

export const pingPreference = keyed([
    { id: 'liberally', label: 'Liberally', color: 'success' },
    { id: 'sparingly', label: 'Sparingly', color: 'warning' },
    { id: 'hints', label: 'Hints', color: 'warning' },
    { id: 'see_notes', label: 'See notes', color: 'info' },
    { id: 'never', label: 'Never', color: 'danger' },
]);
