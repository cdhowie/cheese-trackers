<script setup>
import { ref } from 'vue';
import Modal from './Modal.vue';

defineEmits(['claimed']);

const props = defineProps(['disabled']);

const modalShown = ref(false);
</script>

<template>
    <button
        class="btn btn-sm btn-outline-secondary"
        :disabled="props.disabled"
        @click.prevent="modalShown = true"
    >
        Claim
    </button>

    <Teleport v-if="modalShown" to="body">
        <Modal
            @modalclosed="modalShown = false"
            title="Slot already claimed"
        >
            <slot/>
            <template #buttons="{ hide }">
                <button
                    class="btn btn-secondary"
                    @click.prevent="hide()"
                >Close</button>
                <button
                    class="btn btn-warning"
                    :disabled="props.disabled"
                    @click.prevent="$emit('claimed'); hide();"
                >Claim anyway</button>
            </template>
        </Modal>
    </Teleport>
</template>
