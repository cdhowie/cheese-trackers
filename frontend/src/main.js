//import './assets/main.css'

import { createApp } from 'vue'
import App from './App.vue'
import router from './router'

import { createJsError } from './api';
import { settings } from './settings';

const app = createApp(App)

app.use(router)

app.mount('#app')

window.onerror = async (event, source, lineno, colno, error) => {
    try {
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
