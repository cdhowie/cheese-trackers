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

export function makeFilenameSafe(name) {
    return name.replace(/[<>:"/\\|?*]/g, '_');
}

export function asciiSafeJsonStringify(value) {
    return JSON.stringify(value).replace(
        /[^\x20-\x7F]/g,
        (x) => "\\u" + ("000" + x.codePointAt(0).toString(16)).slice(-4)
    );
}
