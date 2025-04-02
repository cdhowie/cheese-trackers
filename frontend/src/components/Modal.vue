<script setup>
import { onMounted, onUnmounted, useTemplateRef, watch } from 'vue';

const props = defineProps(['title', 'message']);

const emit = defineEmits(['modalclosed']);

const modalElement = useTemplateRef('modal-div');

let modal;

function emitClosed() {
    emit('modalclosed');
}

watch(modalElement, (e, old) => {
    if (old) {
        old.removeEventListener('hidden.bs.modal', emitClosed);
    }

    if (modal) {
        modal.dispose();
        modal = undefined;
    }

    if (e) {
        modal = new window.bootstrap.Modal(e);
        e.addEventListener('hidden.bs.modal', emitClosed);
    }
});

watch(() => props.message, (m) => {
    if (m !== undefined && m !== '') {
        modal?.show();
    } else {
        modal?.hide();
    }
});

onUnmounted(() => {
    modal?.dispose();
});
</script>

<template>
    <div class="modal fade" ref="modal-div">
        <div class="modal-dialog">
            <div class="modal-content">
                <div class="modal-header">
                    <h1 class="modal-title fs-3">{{ props.title }}</h1>
                </div>
                <div class="modal-body">{{ props.message }}</div>
                <div class="modal-footer">
                    <button type="button" class="btn btn-primary" @click="modal.hide()">Close</button>
                </div>
            </div>
        </div>
    </div>
</template>
