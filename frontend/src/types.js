import { keyBy } from "lodash-es";

function keyed(v) {
    v.byId = keyBy(v, 'id');
    return v;
}

export const progressionStatus = keyed([
    { id: 'unknown', label: 'Unknown', color: 'secondary', icon: 'question-lg' },
    { id: 'unblocked', label: 'Unblocked', color: 'light', icon: 'person-walking' },
    { id: 'bk', label: 'BK', color: 'danger', icon: 'octagon-fill' },
    { id: 'go', label: 'Go mode', color: 'success', icon: 'flag' },
]);

export const completionStatus = keyed([
    { id: 'incomplete', label: 'Incomplete', color: 'light', icon: 'square' },
    { id: 'all_checks', label: 'All checks', color: 'info', icon: 'check-square' },
    { id: 'goal', label: 'Goal', color: 'info', icon: 'flag' },
    { id: 'done', label: 'Done', color: 'success', icon: 'flag-fill' },
    { id: 'released', label: 'Released', color: 'secondary', icon: 'escape' },
]);

export const availabilityStatus = keyed([
    { id: 'unknown', label: 'Unknown', color: 'secondary', icon: 'question-lg' },
    { id: 'open', label: 'Open', color: 'success', icon: 'person' },
    { id: 'claimed', label: 'Claimed', color: 'light', icon: 'person-fill' },
    { id: 'public', label: 'Public', color: 'info', icon: 'people-fill' },
]);

export const hintClassification = keyed([
    { id: 'unknown', label: 'Unknown', color: 'light', icon: 'question-lg' },
    { id: 'critical', label: 'Critical', color: 'danger', icon: 'exclamation-triangle-fill' },
    { id: 'useful', label: 'Useful', color: 'warning', icon: 'person-raised-hand' },
    { id: 'trash', label: 'Trash', color: 'secondary', icon: 'trash-fill' },
]);

export const pingPreference = keyed([
    { id: 'liberally', label: 'Liberally', color: 'success' },
    { id: 'sparingly', label: 'Sparingly', color: 'warning' },
    { id: 'hints', label: 'Hints', color: 'warning' },
    { id: 'see_notes', label: 'See notes', color: 'info' },
    { id: 'never', label: 'Never', color: 'danger' },
]);
