<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import type { Transaction } from "@/types";
import EventBus from "@/lib/event_bus";

const num_items = ref(5);
const transactions = ref(new Array<Transaction>());

onMounted(() => {
  EventBus.on("trans", (trans) => {
    if (num_items.value > 0) {
      addTransaction(trans as Transaction);
    }
  });

  // get setting from local storage
  let item = localStorage.getItem("plw_items");
  if (item != null) {
    num_items.value = parseInt(item);
  }
});

function addTransaction(t: Transaction) {
  if (transactions.value.length >= num_items.value) {
    transactions.value = transactions.value.splice(0, num_items.value - 1);
  }
  transactions.value.unshift(t);
}

watch(num_items, async (newVal) => {
  let str = newVal.toString();
  if (str.length != 0) {
    localStorage.setItem("plw_items", str);
  }
  // update log if needed
  if (transactions.value.length > newVal) {
    transactions.value = transactions.value.splice(0, newVal);
  }
});
</script>

<template>
  <div class="box t">
    <h3 class="title is-3">Transaction Log</h3>
    <div class="field has-addons">
      <p class="control">
        <input
          class="input is-small is-rounded"
          v-model.number="num_items"
          type="number"
          min="0"
        />
      </p>
      <p class="control">
        <a class="button is-small is-rounded is-static"> items to show </a>
      </p>
    </div>
    <div class="table-container">
      <table class="table is-narrow is-striped">
        <thead>
          <tr>
            <th>Time [ns]</th>
            <th>Command</th>
            <th>Initiator</th>
            <th>Target</th>
            <th>Address</th>
            <th>Data length</th>
            <th>Data</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in transactions">
            <td>{{ t.sim_time }}</td>
            <td>{{ t.action === 0 ? "Read" : "Write" }}</td>
            <td>{{ t.initiator }}</td>
            <td class="target">{{ t.target }}</td>
            <td class="target">{{ t.address.toString(16) }}</td>
            <td>{{ t.data_length }}</td>
            <td class="target">{{ t.data.toString(16) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style>
input[type="number"] {
  appearance: textfield;
  text-align: center;
}

.target {
  width: 150px;
}
</style>
