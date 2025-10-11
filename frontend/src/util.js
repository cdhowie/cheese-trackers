import { isFinite } from "lodash-es";

export function percent(num, den) {
    num = +num;
    den = +den;
    if (isFinite(num) && isFinite(den) && den !== 0) {
        return (num / den) * 100;
    }

    return 0;
}

export function synchronize(target, source) {
    for (const key of Object.keys(target)) {
        delete target[key];
    }

    Object.assign(target, source);
}

export function roundDown(num, places) {
    const factor = Math.pow(10, places);

    return Math.floor(num * factor) / factor;
}
