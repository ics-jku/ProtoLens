<script setup lang="ts">
import { onMounted, ref, inject, type Ref } from "vue";
import type { AppState } from "@/types";
import type { Configuration, Flag } from "@/lib/tvm/configuration";
import EventBus from "@/lib/event_bus";

const VP_STORAGE_KEY = "plw_config";
const APP_STORAGE_KEY = "plw_app";
const worker: Worker | undefined = inject("worker");
const isOpen = ref(false);
const isVPConfOpen = ref(true);
const props = defineProps<{ appState: AppState }>();

const appConfig: Ref<Flag[]> = ref([
  {
    name: "Drop transaction queue when Stop is pressed",
    active: true,
    value: "Helps with desync when VP is stopped",
  },
  {
    name: "Hide menubar when starting VP",
    active: true,
    value: "Hover over the top region to show the menubar again",
  },
]);

// This is the default configuration loaded after refresh
const conf: Configuration = {
  vp: "riscv-vp",
  proj: "simple-sensor",
  args: "",
  flags: ref([
    { name: "Intercept Syscalls", active: true, value: "--intercept-syscalls" },
    {
      name: "Error on zero traphandler",
      active: true,
      value: "--error-on-zero-traphandler=true",
    },
    { name: "Instruction DMI", active: false, value: "--use-instr-dmi" },
    { name: "Data DMI", active: true, value: "--use-data-dmi" },
  ]),
  quantum: 10000,
  arch: "rv32",
};
const gdb = ref("");
const arch = ["rv32", "rv64"];
const constArgs = "--debug-bus-mode";
const isDebug = ref(false);
const breakOnTrans = ref(false);
const servAddr = ref(
  import.meta.env.DEV ? "localhost:8080" : self.location.host,
);

function connect() {
  worker?.postMessage({ type: "NEW", payload: servAddr.value });
}

function configure() {
  storeConfig();
  connect();
  EventBus.emit("configure", getConfig());
}

onMounted(() => {
  EventBus.on("openConfiguration", () => {
    isOpen.value = true;
  });
  // send configure event after websocket connection is established
  worker?.addEventListener("message", (msg) => {
    switch (msg.data.type) {
      case "open":
        configure();
        break;
      case "options":
        gdb.value = "127.0.0.1:" + msg.data.payload;
        break;
    }
  });
  loadConfig();
  setTimeout(onAppConfigChange, 5);
  connect();
});

function storeConfig() {
  let conf_str = conf.vp + " " + conf.proj + " " + conf.args + " ";
  conf_str += conf.flags.value.map((e) => e.active).join(" ") + " ";
  conf_str += conf.quantum + " " + conf.arch + " " + isDebug.value;
  localStorage.setItem(VP_STORAGE_KEY, conf_str);
}

function loadConfig() {
  let conf_str = localStorage.getItem(VP_STORAGE_KEY);
  let parts = conf_str?.split(" ");
  if (conf_str === undefined || parts === undefined || parts.length < 10) {
    console.log("Could not load config from local storage");
    return;
  }
  conf.vp = parts[0];
  conf.proj = parts[1];
  conf.args = parts[2];
  conf.flags.value[0].active = parts[3] === "true";
  conf.flags.value[1].active = parts[4] === "true";
  conf.flags.value[2].active = parts[5] === "true";
  conf.flags.value[3].active = parts[6] === "true";
  conf.quantum = parseInt(parts[7]);
  conf.arch = parts[8];
  isDebug.value = parts[9] === "true";

  let app_conf = localStorage.getItem(APP_STORAGE_KEY);
  let app_parts = app_conf?.split(" ");
  if (
    app_conf === undefined ||
    app_parts === undefined ||
    app_parts.length != appConfig.value.length
  ) {
    console.log("Could not load app config");
    return;
  }
  for (let i = 0; i < appConfig.value.length; i++) {
    appConfig.value[i].active = app_parts[i] === "true";
  }
}

function closeModal(save: boolean) {
  isOpen.value = false;
  if (save) {
    configure();
  }
}

function onAppConfigChange() {
  localStorage.setItem(
    APP_STORAGE_KEY,
    appConfig.value.map((e) => e.active).join(" "),
  );
  EventBus.emit("appconf", appConfig.value);
}

function getConfig(): Array<string> {
  let args = constArgs;
  if (conf.args != "") {
    args += " " + conf.args;
  }
  if (isDebug.value) {
    args += " --debug-mode";
    if (breakOnTrans.value) {
      args += " --break-on-transaction";
    }
  }
  for (let i = 0; i < conf.flags.value.length; i++) {
    if (conf.flags.value[i].active) {
      args += " " + conf.flags.value[i].value;
    }
  }
  if (conf.quantum > 0) {
    args += " " + "--tlm-global-quantum=" + conf.quantum;
  }
  return new Array<string>(
    conf.vp,
    conf.proj,
    args,
    conf.arch,
    breakOnTrans.value.toString(),
  );
}
</script>

<template>
  <div
    class="modal"
    :class="{ 'is-active': isOpen }"
    tabindex="0"
    @keydown.esc="closeModal(false)"
  >
    <div class="modal-background"></div>
    <div class="modal-card">
      <header class="modal-card-head">
        <p class="modal-card-title">Configuration</p>
        <button
          class="delete"
          aria-label="close"
          @click="closeModal(false)"
        ></button>
      </header>
      <section class="modal-card-body">
        <div class="tabs">
          <ul>
            <li
              :class="{ 'is-active': isVPConfOpen }"
              @click="isVPConfOpen = true"
            >
              <a>Virtual Prototype</a>
            </li>
            <li
              :class="{ 'is-active': !isVPConfOpen }"
              @click="isVPConfOpen = false"
            >
              <a>Application</a>
            </li>
          </ul>
        </div>
        <div v-show="isVPConfOpen">
          <label class="label">Server address</label>
          <div class="field has-addons" v-if="!props.appState.hasSocket">
            <div class="control">
              <input class="input" type="text" v-model="servAddr" />
            </div>
            <div class="control">
              <button class="button is-dark is-danger" @click="connect()">
                Connect
              </button>
            </div>
          </div>
          <div class="field">
            <label class="label">Virtual Prototype</label>
            <div class="select is-fullwidth">
              <select v-model="conf.vp">
                <option disabled value="">Select VP for execution</option>
                <option v-for="vp in props.appState.workingVps">
                  {{ vp }}
                </option>
              </select>
            </div>
            <p class="help">Virtual Prototype to start</p>
          </div>
          <div class="field">
            <label class="label">Binaries</label>
            <div class="select is-fullwidth">
              <select v-model="conf.proj">
                <option disabled value="">Select working directories</option>
                <option v-for="bin in props.appState.workingDirs">
                  {{ bin }}
                </option>
              </select>
            </div>
            <p class="help">
              Name of the working directory which contains the binary and source
              files
            </p>
          </div>
          <div class="field">
            <label class="label is-size-5">Arguments</label>
          </div>
          <div class="field has-addons buttons">
            <div class="control" v-if="isDebug">
              <button
                class="button is-info is-responsive is-outlined"
                @click="isDebug = !isDebug"
              >
                Debug Mode
              </button>
            </div>
            <div class="control" v-else>
              <button
                class="button is-info is-responsive is-outlined"
                @click="isDebug = !isDebug"
              >
                Trace Mode
              </button>
            </div>
            <div class="control">
              <button
                class="button is-responsive is-outlined"
                :class="{
                  'is-primary': breakOnTrans,
                  'is-danger': !breakOnTrans,
                }"
                v-if="isDebug"
                @click="breakOnTrans = !breakOnTrans"
              >
                Break on transaction
              </button>
            </div>
          </div>
          <div class="field buttons are-small">
            <div class="control" v-for="flag in conf.flags.value">
              <button
                class="button is-outlined"
                :class="{
                  'is-primary': flag.active,
                  'is-danger': !flag.active,
                }"
                @click="flag.active = !flag.active"
              >
                {{ flag.name }}
              </button>
            </div>
          </div>
          <div class="field has-addons">
            <div class="control">
              <button class="button is-info is-outlined is-small">
                TLM Global Quantum
              </button>
            </div>
            <div class="control">
              <input
                class="input is-info is-small"
                v-model.number="conf.quantum"
                type="number"
                min="10"
              />
            </div>
          </div>
          <div class="field">
            <div><label class="label">Extra Arguments</label></div>
            <div>
              <div class="control">
                <input class="input" type="text" v-model="conf.args" />
              </div>
              <p class="help">
                Additional arguments passed to the Virtual Prototype
              </p>
            </div>
          </div>
          <div v-show="isDebug">
            <div class="field">
              <label class="label">Architecture</label>
              <div class="select is-fullwidth">
                <select v-model="conf.arch">
                  <option disabled value="">Select target architecture</option>
                  <option v-for="bin in arch">{{ bin }}</option>
                </select>
              </div>
              <p class="help">Set architecture of the started debugger</p>
            </div>
            <div class="field columns">
              <div v-if="gdb != ''" class="column">
                <label class="label">GDB Proxy</label>
              </div>
              <div class="column">{{ gdb }}</div>
            </div>
          </div>
          <div class="field is-grouped is-grouped-right">
            <div class="control">
              <button
                class="button is-dark is-primary"
                @click="closeModal(true)"
              >
                Save
              </button>
            </div>
          </div>
        </div>
        <div v-show="!isVPConfOpen">
          <div class="field" v-for="flag in appConfig">
            <button
              class="button"
              :class="{ 'is-primary': flag.active, 'is-danger': !flag.active }"
              @click="
                flag.active = !flag.active;
                onAppConfigChange();
              "
            >
              {{ flag.name }}
            </button>
            <p class="help">{{ flag.value }}</p>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped></style>
