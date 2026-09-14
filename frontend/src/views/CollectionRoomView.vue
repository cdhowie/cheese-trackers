<script setup>
import { computed, ref, watch } from 'vue';
import moment from 'moment';
import { filter, forEach, groupBy, join, map, orderBy } from 'lodash-es';
import JSZip from 'jszip';

import { createCollectionRoomSlots, deleteCollectionRoomSlot, getCollectionRoom, getCollectionRoomSlots, updateCollectionRoom } from '@/api';
import Repeat from '@/components/Repeat.vue';
import { settings } from '@/settings';
import { now } from '@/time';
import DateTimeEdit from '@/components/DateTimeEdit.vue';
import DownloadLink from '@/components/DownloadLink.vue';
import { makeFilenameSafe } from '@/util';
import CollectionRoomBadges from '@/components/CollectionRoomBadges.vue';

const props = defineProps(['roomid']);

const room = ref(undefined);
const roomSlots = ref(undefined);
const loading = ref(false);
const error = ref(undefined);

const editRoom = ref(undefined);
const saving = ref(false);
const saveError = ref(undefined);

const busy = computed(() => loading.value || saving.value);

const isOwner = computed(() =>
  room.value &&
  room.value.owner_ct_user_id === settings.value.auth?.userId
);

const isClosed = computed(() =>
  room.value && (
    room.value.is_closed || (
      room.value.closes_at && moment(room.value.closes_at).isSameOrBefore(now.value)
    )
  )
);

const sortedSlots = computed(() => orderBy(roomSlots.value, 'slot_name'));

const allSlotsYaml = computed(() =>
  join(
    map(sortedSlots.value, (s) => s.yaml),
    "---\n"
  )
);

const allSlotsZip = ref(undefined);
let allSlotsZipSerial = 0;

watch(roomSlots, async () => {
  allSlotsZip.value = undefined;

  if (!roomSlots.value) {
    return;
  }

  allSlotsZipSerial += 1;
  const serial = allSlotsZipSerial;

  const zip = new JSZip();

  forEach(roomSlots.value, (slot, idx) => {
    zip.file(
      makeFilenameSafe(`${room.value.title} - ${idx + 1} - ${slot.slot_name}.yaml`),
      slot.yaml
    );
  });

  const result = await zip.generateAsync({
    type: 'blob',
    compression: 'DEFLATE',
    compressionOptions: { level: 6 },
  });

  if (serial === allSlotsZipSerial) {
    allSlotsZip.value = result;
  }
});

const slotsByPlayerZip = ref(undefined);
let slotsByPlayerZipSerial = 0;

watch(roomSlots, async () => {
  slotsByPlayerZip.value = undefined;

  if (!roomSlots.value) {
    return;
  }

  slotsByPlayerZipSerial += 1;
  const serial = slotsByPlayerZipSerial;

  const zip = new JSZip();

  forEach(
    groupBy(roomSlots.value, (s) => s.owner_discord_username),
    (slots, player) => {
      const yaml = join(
        map(slots, (s) => s.yaml),
        "---\n"
      );

      zip.file(makeFilenameSafe(`${room.value.title} - ${player}.yaml`), yaml);
    }
  );

  const result = await zip.generateAsync({
    type: 'blob',
    compression: 'DEFLATE',
    compressionOptions: { level: 6 },
  });

  if (serial === slotsByPlayerZipSerial) {
    slotsByPlayerZip.value = result;
  }
});

function updateLocalRoom(data) {
  room.value = data;

  editRoom.value = {
    id: data.id,
    title: data.title,
    is_closed: data.is_closed,
    closes_at: data.closes_at && moment(data.closes_at),
  };  
}

async function uploadFiles(files) {
  if (!files?.length || saving.value) {
    return;
  }

  saving.value = true;
  saveError.value = undefined;

  try {
    for (const file of files) {

      const { data } = await createCollectionRoomSlots(room.value.id, file);

      roomSlots.value = [...roomSlots.value, ...data];
    }
  } catch (e) {
    saveError.value = e;
  } finally {
    saving.value = false;
  }
}

async function deleteSlot(slot) {
  if (saving.value) {
    return;
  }

  saving.value = true;
  saveError.value = undefined;

  try {
    const { data } = await deleteCollectionRoomSlot(room.value.id, slot.id);

    roomSlots.value = filter(roomSlots.value, (s) => s.id !== data.id);
  } catch (e) {
    saveError.value = e;
  } finally {
    saving.value = false;
  }
}

async function updateRoom() {
  if (busy.value) {
    return;
  }

  saving.value = true;
  saveError.value = undefined;

  try {
    const { data } = await updateCollectionRoom({
      ...editRoom.value,
      closes_at: editRoom.value.closes_at && editRoom.value.closes_at.toISOString(),
    });

    updateLocalRoom(data);
  } catch (e) {
    saveError.value = e;

    // This will reset the edit state.
    updateLocalRoom(room.value);
  } finally {
    saving.value = false;
  }
}

async function loadRoom() {
  if (loading.value) {
    return;
  }

  loading.value = true;
  error.value = undefined;

  try {
    const [r, s] = await Promise.all([
      getCollectionRoom(props.roomid),
      getCollectionRoomSlots(props.roomid),
    ]);

    updateLocalRoom(r.data);
    roomSlots.value = s.data;
  } catch (e) {
    error.value = e;
  } finally {
    loading.value = false;
  }
}

loadRoom();

watch(() => props.roomid, () => loadRoom());
</script>

<template>
  <div class="container">
    <table v-if="loading" class="table placeholder-wave">
      <thead>
        <tr>
          <th><div class="placeholder w-100"></div></th>
          <th><div class="placeholder w-100"></div></th>
          <th><div class="placeholder w-100"></div></th>
        </tr>
      </thead>
      <tbody>
        <Repeat times="10">
          <tr>
            <td><div class="placeholder bg-secondary w-100"></div></td>
            <td><div class="placeholder bg-secondary w-100"></div></td>
            <td><div class="placeholder bg-secondary w-100"></div></td>
          </tr>
        </Repeat>
      </tbody>
    </table>
    <div v-else-if="error" class="text-danger">
      Error loading room: {{ error }}
    </div>
    <div v-else>
      <h2 class="text-center">
        <span>{{ room.title }}</span> by {{ room.owner_discord_username }}
      </h2>

      <div class="d-flex gap-1 justify-content-center">
        <CollectionRoomBadges :room="room"/>
      </div>

      <div v-if="saveError" class="alert alert-danger">Failed to save changes: {{ saveError }}</div>

      <form class="container bg-dark-subtle rounded pt-3 mb-4 mt-4" v-if="isOwner">
        <div class="row">
          <div class="col-12 mb-3">
            <label for="collectionRoomTitle" class="form-label">Title</label>
            <input id="collectionRoomTitle" class="form-control" type="text" placeholder="Title"
              v-model="editRoom.title"
              @blur="updateRoom"
              :disabled="busy">
          </div>

          <div class="col-12 col-lg-6 mb-3">
            <label for="collectionRoomIsClosed" class="form-label">Closed</label>
            <div class="btn-group form-control border-0 p-0">
              <input type="radio" class="btn-check" name="collectionRoomIsClosed" id="collectionRoomIsClosedNo"
                v-model="editRoom.is_closed" :value="false" :disabled="busy" @change="updateRoom">
              <label class="btn btn-outline-success" for="collectionRoomIsClosedNo">No</label>

              <input type="radio" class="btn-check" name="collectionRoomIsClosed" id="collectionRoomIsClosedYes"
                v-model="editRoom.is_closed" :value="true" :disabled="busy" @change="updateRoom">
              <label class="btn btn-outline-warning" for="collectionRoomIsClosedYes">Yes</label>
            </div>
          </div>

          <div class="col-12 col-lg-6 mb-3">
            <label for="collectionRoomClosesAt" class="form-label">Closes at</label>
            <div class="input-group">
              <DateTimeEdit id="collectionRoomClosesAt" class="form-control" v-model="editRoom.closes_at" @blur="updateRoom"/>
              <button class="btn btn-outline-secondary" type="button" @click.prevent="editRoom.closes_at = undefined; updateRoom();">
                <i class="bi-trash"/>
              </button>
            </div>
          </div>
        </div>
      </form>

      <form v-if="!isClosed && settings.auth?.userId" class="mb-4">
        <label for="collectionRoomUpload" class="form-label">Upload slot(s)</label>
        <input name="collectionRoomUpload" type="file" multiple class="form-control" @change="
          uploadFiles($event.target.files);
          $event.target.value = '';
        ">
      </form>

      <div class="text-end" v-if="roomSlots?.length">
        <DownloadLink
          class="btn btn-primary"
          :content="allSlotsYaml"
          content-type="application/yaml"
          :filename="`${room.title} slots.yaml`"
        ><i class="bi-download"/> Download single YAML</DownloadLink>
        <DownloadLink
          class="btn btn-primary ms-1"
          :class="{ disabled: !slotsByPlayerZip }"
          :content="slotsByPlayerZip"
          content-type="application/zip"
          :filename="`${room.title} slots.zip`"
        ><i class="bi-download"/> Download ZIP (file per player)</DownloadLink>
        <DownloadLink
          class="btn btn-primary ms-1"
          :class="{ disabled: !allSlotsZip }"
          :content="allSlotsZip"
          content-type="application/zip"
          :filename="`${room.title} slots.zip`"
        ><i class="bi-download"/> Download ZIP (file per slot)</DownloadLink>
      </div>

      <table class="table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Game</th>
            <th>Owner</th>
            <th/>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(slot, idx) in sortedSlots" :key="slot.id">
            <td>{{ slot.slot_name }}</td>
            <td>{{ slot.slot_game }}</td>
            <td>{{ slot.owner_discord_username }}</td>
            <td>
              <div class="btn-group">
                <DownloadLink
                  class="btn btn-sm btn-primary"
                  :content="slot.yaml"
                  :filename="`${room.title} - ${idx + 1} - ${slot.slot_name}.yaml`"
                  content-type="application/yaml"
                ><i class="bi-download"/></DownloadLink>
                <button
                  class="btn btn-sm btn-danger"
                  :disabled="saving"
                  v-if="isOwner || (!isClosed && slot.owner_ct_user_id === settings.auth?.userId)"
                  @click.prevent="deleteSlot(slot)"
                ><i class="bi-trash"/></button>
              </div>
            </td>
          </tr>
          <tr v-if="!roomSlots?.length">
            <td colspan="4" class="text-center text-secondary">
              There are no slots yet.
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
