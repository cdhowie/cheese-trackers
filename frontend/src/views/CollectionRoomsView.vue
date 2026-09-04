<script setup>
import { computed, ref } from 'vue';

import Repeat from '@/components/Repeat.vue';

import { createCollectionRoom, getUserCollectionRooms } from '@/api';
import moment from 'moment';
import { trim } from 'lodash-es';
import router from '@/router';
import { RouterLink } from 'vue-router';
import DateTimeEdit from '@/components/DateTimeEdit.vue';

const rooms = ref([]);
const loading = ref(false);
const error = ref(undefined);

async function loadRooms() {
  if (loading.value) {
    return;
  }

  loading.value = true;
  error.value = undefined;

  try {
    rooms.value = (await getUserCollectionRooms()).data;
  } catch (e) {
    error.value = e;
  } finally {
    loading.value = false;
  }
}

loadRooms();

const newTitle = ref("");
const newClosure = ref(undefined);

const isNewValid = computed(() =>
  trim(newTitle.value).length && (
    newClosure.value === undefined || newClosure.value.isValid()
  )
);

const createLoading = ref(false);
const createError = ref(undefined);

async function createNewRoom() {
  if (createLoading.value) {
    return;
  }

  createLoading.value = true;
  createError.value = undefined;

  try {
    const { data } = await createCollectionRoom({
      title: newTitle.value,
      closes_at: newClosure.value?.toISOString(),
      is_closed: false,
    });

    router.push(`/collection_room/${data.id}`);
  } catch (e) {
    createError.value = e;
  } finally {
    createLoading.value = false;
  }
}
</script>

<template>
  <div class="container">
    <h2 class="text-center">Collection Rooms</h2>

    <form class="row">
      <div class="col-12 col-lg-6">
        <label for="createcollectionroomtitle" class="form-label">Title</label>
        <input id="createcollectionroomtitle" class="form-control" type="text" placeholder="Title" v-model="newTitle">
      </div>
      <div class="col-12 col-lg-6">
        <label for="createcollectionroomclosuredate" class="form-label">Close at</label>
        <div class="input-group">
          <DateTimeEdit id="createcollectionroomclosuredate" class="form-control" v-model="newClosure"/>
          <button class="btn btn-outline-secondary" type="button" v-if="newClosure" @click.prevent="newClosure = undefined">
            <i class="bi-trash"/>
          </button>
        </div>
      </div>
      <div class="col-12 mt-2">
        <button
          class="btn btn-primary form-control"
          :disabled="createLoading || !isNewValid"
          @click.prevent="createNewRoom"
        >Create</button>
      </div>
      <div class="col-12 mt-2 text-danger" v-if="createError">
        Error creating room: {{ createError }}
      </div>
    </form>

    <table v-if="loading" class="table placeholder-wave">
      <thead>
        <tr>
          <th><div class="placeholder w-100"></div></th>
        </tr>
      </thead>
      <tbody>
        <Repeat times="3">
          <tr>
            <td><div class="placeholder bg-secondary w-100"></div></td>
          </tr>
        </Repeat>
      </tbody>
    </table>
    <div v-else-if="error" class="text-danger">
      Error loading rooms: {{ error }}
    </div>
    <table v-else class="table">
      <thead>
        <tr>
          <th>Name</th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="rooms.length === 0">
          <td colspan="1" class="text-center text-secondary">
            You do not have any collection rooms.
          </td>
        </tr>
        <tr v-for="room in rooms">
          <td>
            <RouterLink :to="`/collection_room/${room.id}`">{{ room.title }}</RouterLink>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
