<script setup lang="ts">
import ControlBar from "@/components/misc/ControlBar.vue";
import Configuration from "@/components/misc/Configuration.vue";
import SnackBar from "@/components/misc/SnackBar.vue";
import Framebuffer from "@/components/tvm/Framebuffer.vue";
import TransactionLog from "@/components/tvm/TransactionLog.vue";
import Architecture from "@/components/tvm/Architecture.vue";
import Terminal from "@/components/tvm/Terminal.vue";

import { onMounted, ref, inject } from "vue";
import type { AppState } from "@/types";
import parseBinary from "@/lib/transaction_parser";
import EventBus from "@/lib/event_bus";

const worker: Worker | undefined = inject("worker");

const state: AppState = {
  // Data structures for TVMs
  modules: new Array<string>(),
  start_addrs: new Array<string>(),
  end_addrs: new Array<string>(),
  // VPs and Source dirs loaded by the server
  workingDirs: new Array<string>(),
  workingVps: new Array<string>(),
  // current application state
  hasSocket: ref(false),
  hasVP: ref(false),
  hasGdb: ref(false),
};

const activeTvms = ref(new Array<string>());
const reset = ref(true);
let perf_cnt = 0;

onMounted(() => {
  initializeWorker();
  setInterval(() => {
    EventBus.emit("perf_trans", perf_cnt);
    perf_cnt = 0;
  }, 1000);
});

// react to webworkers websocket and server data
function initializeWorker() {
  worker?.addEventListener("message", (msg) => {
    let out = "";
    let kind = 4;
    switch (msg.data.type) {
      case "open":
        state.hasSocket.value = true;
      case "close":
      case "error":
        state.hasSocket, (state.hasVP.value = false);
        break;
      case "config":
        state.workingDirs = msg.data.payload.dirs;
        state.workingVps = msg.data.payload.vps;
        break;
      case "layout":
        state.modules = msg.data.payload.modules;
        state.start_addrs = msg.data.payload.start_addrs;
        state.end_addrs = msg.data.payload.end_addrs;
        activateTVMs();
      case "status":
        // -1 - lost connection
        // 0 - no state change
        // 1 - new connection
        if (msg.data.payload.vp > 0) {
          out = "VP running";
          state.hasVP.value = true;
          kind = 1;
        }
        if (msg.data.payload.vp < 0) {
          out = "VP not running";
          state.hasVP.value = false;
        }
        if (msg.data.payload.gdb > 0) {
          out = "gdbgui connected";
          state.hasGdb.value = true;
          kind = 1;
        }
        if (msg.data.payload.gdb < 0) {
          out = "gdbgui not connected";
          state.hasGdb.value = false;
        }
        //snackBar.value.show(out, kind)
        break;
      case "start":
        if (msg.data.payload === "true") {
          out = "VP started";
          state.hasVP.value = true;
          kind = 3;
          resetState();
        } else if (msg.data.payload === "false") {
          out = "VP start failed";
          state.hasVP.value = false;
          kind = 5;
        } else {
          out = msg.data.payload;
          state.hasVP.value = false;
          kind = 3;
        }
        //snackBar.value.show(out, kind)
        break;
      case "bin":
        let trans = parseBinary(msg.data.payload, state.modules);
        perf_cnt += trans.length;
        trans.forEach((e) => EventBus.emit("trans", e));
    }
  });
}

// Activates all TVMs which are present on the current VP
// This is the connecting point between TVM name and SystemC module name
function activateTVMs() {
  let mods = state.modules.map((e) => e.toLocaleLowerCase());
  mods.forEach((mod) => {
    if (mod.indexOf("fb") != -1) {
      activeTvms.value.push("Framebuffer");
    } else if (mod.indexOf("simpleterminal") != -1) {
      activeTvms.value.push("Terminal");
    }
  });
  setTimeout(() => {
    EventBus.emit("TVM");
  }, 10);
}

// This method disables rendering for a short amount of time
// therefore Vue reinstatiates all components
function resetState() {
  state.modules.splice(0, state.modules.length);
  state.start_addrs.splice(0, state.start_addrs.length);
  state.end_addrs.splice(0, state.end_addrs.length);
  activeTvms.value.splice(0, activeTvms.value.length);
  reset.value = false;
  setTimeout(() => {
    reset.value = true;
  }, 1);
}
</script>

<template>
  <main>
    <Configuration :appState="state" />
    <ControlBar :appState="state" />
    <div v-if="reset">
      <div>
        <Architecture :appState="state" />
      </div>
      <div class="columns">
        <div class="column">
          <Terminal v-if="activeTvms.indexOf('Terminal') != -1" />
          <Framebuffer
            v-if="activeTvms.indexOf('Framebuffer') != -1"
            :appState="state"
          ></Framebuffer>
        </div>
        <div class="column">
          <TransactionLog />
        </div>
      </div>
    </div>
    <SnackBar></SnackBar>
  </main>
</template>

<style scoped></style>
