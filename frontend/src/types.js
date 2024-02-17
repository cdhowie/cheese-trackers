import { keyBy } from "lodash-es";

export const gameStatus = [
    { id: 'unblocked', label: 'Unblocked', color: 'light' },
    { id: 'bk', label: 'BK', color: 'danger' },
    { id: 'all_checks', label: 'All checks', color: 'warning' },
    { id: 'done', label: 'Done', color: 'success' },
    { id: 'open', label: 'Open', color: 'info' },
    { id: 'released', label: 'Released', color: 'secondary' },
    { id: 'glitched', label: 'Glitched', color: 'secondary' },
];

gameStatus.byId = keyBy(gameStatus, 'id');

export const pingPreference = [
    { id: 'liberally', label: 'Liberally', color: 'success' },
    { id: 'sparingly', label: 'Sparingly', color: 'warning' },
    { id: 'hints', label: 'Hints', color: 'warning' },
    { id: 'see_notes', label: 'See notes', color: 'info' },
    { id: 'never', label: 'Never', color: 'danger' },
];

pingPreference.byId = keyBy(pingPreference, 'id');
