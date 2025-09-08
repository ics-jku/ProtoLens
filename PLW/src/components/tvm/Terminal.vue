<script setup lang="ts">
import EventBus from "@/lib/event_bus";
import type { Transaction } from "@/types";
import { onMounted, ref } from "vue";

const lines = ref(Array<string>(""));
let idx = 0;

onMounted(() => {
  EventBus.on("trans", (trans) => {
    let transaction = trans as Transaction;
    if (transaction.target.toLowerCase().indexOf("terminal") != -1) {
      add(transaction.data);
    }
  });
});

function add(data: bigint) {
  let hex = data.toString(16);
  lines.value[idx] += hexToAscii(hex);
  if (hex.endsWith("0A")) {
    lines.value.push("");
    idx++;
  }
}
function hexToAscii(hex: string) {
  var str = "";
  for (var n = 0; n < hex.length; n += 2) {
    str += String.fromCharCode(parseInt(hex.substring(n, 2), 16));
  }
  return str;
}
</script>

<template>
  <div class="box">
    <h3 class="title is-3">Terminal</h3>
    <p v-for="p in lines">
      {{ p }}
    </p>
  </div>
</template>

<style scoped>
p {
  font-family: "Courier New", Courier, monospace;
}
</style>
