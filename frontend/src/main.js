//import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router'

import 'vue-virtual-scroller/dist/vue-virtual-scroller.css';

import { createJsError } from './api';
import { settings } from './settings';

const app = createApp(App)

app.use(router)

app.mount('#app')

function shouldReportError(error) {
    // Don't report network errors, they are very unlikely to be code bugs.
    return error?.name !== 'AxiosError';
}

window.onerror = async (event, source, lineno, colno, error) => {
    try {
        if (!shouldReportError(error)) {
            return;
        }

        // Don't log errors without a source or that came from an extension.
        // (Errors without a source typically come from extensions, too.)
        if (!source || /(chrome|moz)-extension/.test(source)) {
            return;
        }

        const msg = JSON.stringify({
            event: `${event}`,
            source,
            lineno,
            colno,
            error: `${error}`,
            stack: error.stack,
        });

        await createJsError({
            error: msg,
            ct_user_id: settings.value?.auth?.userId,
        });
    } catch (e) {
        console.log(`Error from unhandled error handler: ${e}`);
    }
};

window.addEventListener('unhandledrejection', async (event) => {
    try {
        if (!shouldReportError(event.reason)) {
            return;
        }

        const msg = JSON.stringify({
            error: `${event.reason}`,
            stack: event.reason?.stack,
        });

        await createJsError({
            error: msg,
            ct_user_id: settings.value?.auth?.userId,
        });
    } catch (e) {
        console.log(`Error from unhandled promise rejection handler: ${e}`);
    }
});
