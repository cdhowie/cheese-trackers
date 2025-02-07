import { ref } from "vue";

export const showCopiedToast = ref(false);

let timeout = undefined;

export function copy(text) {
    navigator.clipboard.writeText(text);

    showCopiedToast.value = true;

    clearTimeout(timeout);

    timeout = setTimeout(() => {
        showCopiedToast.value = false;
        timeout = undefined;
    }, 3000);
}
