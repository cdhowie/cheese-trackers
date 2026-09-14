<script setup>
import { computed } from 'vue';
import moment from 'moment';

import { settings } from '@/settings';
import { now } from '@/time';

const props = defineProps(['room']);

const isOwner = computed(() =>
  props.room &&
  props.room.owner_ct_user_id === settings.value.auth?.userId
);

const isClosed = computed(() =>
  props.room && (
    props.room.is_closed || (
      props.room.closes_at && moment(props.room.closes_at).isSameOrBefore(now.value)
    )
  )
);

const closesAt = computed(() =>
  props.room.closes_at && moment(props.room.closes_at)
);
</script>

<template>
  <span class="badge text-bg-success" v-if="isOwner">
    Yours
  </span>
  <span class="badge text-bg-warning" v-if="isClosed">
    Closed
  </span>
  <span class="badge text-bg-info" v-if="!isClosed && closesAt" :title="closesAt.toLocaleString()">
    Closes {{ closesAt.from(now) }}
  </span>
</template>
