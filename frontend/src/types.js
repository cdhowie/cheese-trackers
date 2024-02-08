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
