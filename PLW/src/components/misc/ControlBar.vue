<script setup lang="ts">
import { ref, inject, onMounted } from "vue";
import EventBus from "@/lib/event_bus";
import type { AppState } from "@/types";

const worker: Worker | undefined = inject("worker");
const props = defineProps<{ appState: AppState }>();
const trans_perf_cnt = ref(0);
const fps_perf_cnt = ref(0);

const steps = ref(100);
const status = ref(new Array<string>(""));
const statusActive = ref(false);
const isEnabled = ref(true);
const showHeight = 25;
let curOpenTimer = -1;

let configuration = Array<string>();
let resetWSonStop: boolean = false;
let hideMenuBar: boolean = false;

onMounted(() => {
  worker?.addEventListener("message", (msg) => {
    switch (msg.data.type) {
      case "config":
        addMessage(
          msg.data.payload.dirs.length +
            " binaries and " +
            msg.data.payload.vps.length +
            " vps detected",
        );
        break;
      case "layout":
        addMessage(msg.data.payload.modules.length + " modules");
        break;
    }
  });
  EventBus.on("perf_trans", (msg: any) => {
    trans_perf_cnt.value = msg;
  });
  EventBus.on("perf_fps", (msg: any) => {
    fps_perf_cnt.value = msg;
  });
  EventBus.on("configure", (config: any) => {
    configuration = config as Array<string>;
  });
  EventBus.on("appconf", (config: any) => {
    resetWSonStop = config[0].active as boolean;
    hideMenuBar = config[1].active as boolean;
  });
});

function changeBarToggle(activate: boolean) {
  if (activate) {
    isEnabled.value = false;
    window.onmousemove = (event: any) => {
      if (event.clientY <= showHeight) {
        isEnabled.value = true;
        clearTimeout(curOpenTimer);
      } else if (event.clientY <= showHeight * 3) {
        clearTimeout(curOpenTimer);
        curOpenTimer = setTimeout(() => {
          isEnabled.value = false;
        }, 2000);
      }
    };
  } else {
    isEnabled.value = true;
    window.onmousemove = null;
  }
}

function step(nStep: Number) {
  let payload = { command: "Step", value: nStep.toString() };
  worker?.postMessage({ type: "MSG", payload: payload });
}

function start() {
  let payload = {
    command: "Start",
    value: JSON.stringify({
      vp: configuration[0],
      proj: configuration[1],
      args: configuration[2],
      gdb_arch: configuration[3],
    }),
  };
  worker?.postMessage({ type: "MSG", payload: payload });
  if (hideMenuBar) {
    changeBarToggle(true);
  }
}

function stop() {
  let payload = { command: "Start", value: "" };
  worker?.postMessage({ type: "MSG", payload: payload });
  if (resetWSonStop) {
    worker?.postMessage({ type: "RST" });
  }
}

function sendOptions(options: string) {
  let payload = { command: "Options", value: options };
  worker?.postMessage({ type: "MSG", payload: payload });
}

function addMessage(message: string) {
  let msg = new Date().toLocaleTimeString();
  status.value.push(msg.substring(0, msg.length - 2) + "   " + message);
}

function openStatus() {
  statusActive.value = !statusActive.value;
}

defineExpose({ sendOptions });
</script>

<template>
  <div v-show="isEnabled" class="box columns is-centered">
    <div class="column">
      <div class="field has-addons">
        <div class="control">
          <button
            class="button is-dark is-primary is-rounded is-responsive"
            @click="EventBus.emit('openConfiguration')"
          >
            Configure
          </button>
        </div>
        <div
          class="control"
          v-if="!props.appState.hasVP.value && props.appState.hasSocket.value"
        >
          <button
            class="button is-dark is-primary is-rounded is-responsive"
            @click="start()"
          >
            Start
          </button>
        </div>
        <div
          class="control"
          v-else-if="
            props.appState.hasVP.value && props.appState.hasSocket.value
          "
        >
          <button
            class="button is-dark is-danger is-rounded is-responsive"
            @click="stop()"
          >
            Stop
          </button>
        </div>
        <div class="control">
          <div class="dropdown is-left" :class="{ 'is-active': statusActive }">
            <div class="dropdown-trigger">
              <button class="button" @click="openStatus()">...</button>
            </div>
            <div class="dropdown-menu" role="menu">
              <div class="dropdown-content">
                <div class="dropdown-item" v-for="msg in status">
                  {{ msg }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div class="column">
      <div class="field has-addons">
        <div class="control">
          <button
            class="button is-dark is-primary is-rounded is-responsive"
            @click="step(1)"
            :disabled="
              !props.appState.hasSocket ||
              !props.appState.hasVP ||
              !props.appState.hasGdb
            "
          >
            Step
          </button>
        </div>
        <div class="control">
          <input
            class="input is-primary is-rounded"
            v-model.number="steps"
            type="number"
            min="2"
          />
        </div>
        <div class="control">
          <button
            class="button is-dark is-primary is-rounded is-responsive"
            @click="step(steps)"
            :disabled="
              !props.appState.hasSocket ||
              !props.appState.hasVP ||
              !props.appState.hasGdb
            "
          >
            Steps
          </button>
        </div>
      </div>
    </div>
    <div class="column is-narrow">
      <div class="control">
        <div class="tags has-addons">
          <span class="tag perf">{{ trans_perf_cnt }}</span>
          <span class="tag">t/s</span>
        </div>
      </div>
      <div class="control">
        <div class="tags has-addons">
          <span class="tag perf">{{ fps_perf_cnt }}</span>
          <span class="tag">f/s</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
input[type="number"] {
  appearance: textfield;
  text-align: center;
}

.perf {
  width: 60px;
}
</style>
