<script setup>
import { getTrackerChecksHistory } from '@/api';
import { map, sum, values } from 'lodash-es';
import moment from 'moment';
import { computed, ref, watch } from 'vue';
import { Line } from 'vue-chartjs';
import { Chart as ChartJS, Title, Tooltip, LineElement, TimeScale, LinearScale, PointElement } from 'chart.js';
import 'chartjs-adapter-moment';

ChartJS.register(Title, Tooltip, LineElement, PointElement, LinearScale, TimeScale);

const props = defineProps(['trackerid', 'refreshserial', 'totalchecks']);

const data = ref(undefined);
const loading = ref(false);
const error = ref(undefined);

const scalecolor = {
  grid: {
    color: '#444',
  },
  ticks: {
    color: '#888',
  },
};

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  color: '#dee2e6',
  borderColor: '#dee2e6',
  scales: {
    x: {
      type: 'time',
      ...scalecolor,
    },
    y: {
      min: 0,
      max: props.totalchecks,
      ...scalecolor,
    },
  },
  plugins: {
    legend: {
      display: false,
    },
  },
}));

watch(
  () => props.trackerid,
  () => loadData(),
);

watch(
  () => props.refreshserial,
  () => loadData(),
);

async function loadData() {
  if (loading.value) {
    return;
  }

  error.value = undefined;

  if (props.trackerid === undefined || props.trackerid === '') {
    data.value = undefined;
    return;
  }

  loading.value = true;
  try {
    const r = await getTrackerChecksHistory(props.trackerid);

    data.value = {
      datasets: [{
        label: 'Checks',
        borderColor: '#198754',
        backgroundColor: '#198754',
        data: map(r.data, (i) => ({
          x: moment(i.time),
          y: sum(values(i.slots)),
        }))
      }],
    };
  } catch (e) {
    data.value = undefined;
    error.value = `${e}`;
  } finally {
    loading.value = false;
  }
}

loadData();
</script>

<template>
  <div v-if="error" class="alert alert-danger">Failed to load checks history: {{ error }}</div>
  <template v-else-if="data">
    <div style="height: 50vh">
      <Line :data="data" :options="chartOptions" />
    </div>
    <div class="text-secondary small">
      Because data is only requested from Archipelago when this tracker is
      loaded or refreshed, there may be gaps in this history.
    </div>
  </template>
  <div v-else-if="loading" class="alert alert-info">Loading checks history...</div>
</template>
