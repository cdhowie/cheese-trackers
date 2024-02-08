import { isFinite } from "lodash-es";

export function percent(num, den) {
    num = +num;
    den = +den;
    if (isFinite(num) && isFinite(den) && den !== 0) {
        return (num / den) * 100;
    }

    return 0;
}