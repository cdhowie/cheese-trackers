import { ref } from "vue";

export const now = ref(new Date());

setInterval(() => { now.value = new Date(); }, 1000);
