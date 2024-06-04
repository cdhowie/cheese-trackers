import { mapValues } from 'lodash-es';

export const breakpoints = mapValues(
    {
        name: "col-12 col-sm-4 col-md-3 col-xl-1",
        ping: "col-6 col-sm-4 col-md-2 col-xl-1",
        availability: "col-6 col-sm-4 col-md-3 col-xl-1",
        owner: "col-6 col-sm-4 col-md-4 col-xl-2",
        game: "col-6 col-sm-4 col-md-2 col-xl-2",
        status: "col-6 col-sm-4 col-md-4 col-xl-2",
        lastactivity: "col-6 col-sm-4 col-md-2 col-xl-1",
        checks: "col-6 col-sm-4 col-md-2 col-xl-1",
        hints: "col-6 col-sm-4 col-md-2 col-xl-1",
    },
    v => `align-self-center p-1 ${v}`
);
