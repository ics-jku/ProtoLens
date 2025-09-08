<script setup lang="ts">
import EventBus from "@/lib/event_bus";
import type { AppState, Transaction } from "@/types";
import { onMounted, ref } from "vue";

const canvas = ref<HTMLCanvasElement>();
const props = defineProps<{ appState: AppState }>();
const w = 800;
const h = 480;

// 4 pixels with 2 bytes per pixel
const buffer = new ArrayBuffer(8);
const dataView = new DataView(buffer, 0);

const divider = BigInt(w * 2);
const two = BigInt(2);

let ctx: CanvasRenderingContext2D | undefined | null = undefined;
let framebuffer: ImageData | undefined = undefined;
let redraw = false;
let start_address: bigint = BigInt(-1);
let pps = ref(0); // pixels/s
let pixel_cnt = 0;

onMounted(() => {
  ctx = canvas.value?.getContext("2d");
  if (ctx == undefined) {
    return;
  }

  setInterval(() => {
    pps.value = pixel_cnt;
    pixel_cnt = 0;
  }, 1000);

  EventBus.on("trans", (trans) => {
    const transaction = trans as Transaction;

    // Check if transaction targets Framebuffer
    if (isFB(transaction.target) && transaction.action === 1) {
      updateFb(transaction);
    }
  });
  EventBus.on("TVM", () => {
    for (let i = 0; i < props.appState.modules.length; i++) {
      if (isFB(props.appState.modules[i])) {
        start_address = BigInt("0x" + props.appState.start_addrs[i]);
      }
    }
  });

  ctx.fillStyle = "white";
  ctx.fillRect(0, 0, w, h);
  framebuffer = ctx.getImageData(0, 0, w, h);
  loop();
});

function loop() {
  if (framebuffer != undefined && ctx != undefined && redraw === true) {
    ctx.putImageData(framebuffer, 0, 0);
    redraw = false;
  }
  requestAnimationFrame(loop);
}

function updateFb(trans: Transaction) {
  if (framebuffer === undefined) {
    return;
  }

  let coords = getCoords(trans.address);
  if (coords === undefined) {
    return;
  }

  dataView.setBigUint64(0, trans.data, false);
  if (trans.data_length < 2) {
    console.log(trans);
  }
  for (let i = 0; i < trans.data_length; i += 2) {
    let pixel = dataView.getUint16(i, true);
    let idx = coords.y * (framebuffer.width * 4) + coords.x * 4;
    framebuffer.data[idx] = (((pixel & 63488) >> 11) * 527 + 23) >> 6;
    framebuffer.data[idx + 1] = (((pixel & 2016) >> 5) * 259 + 33) >> 6;
    framebuffer.data[idx + 2] = ((pixel & 31) * 527 + 23) >> 6;

    if (coords.x >= w) {
      coords.x = 0;
      coords.y += 1;
    } else {
      coords.x += 1;
    }
  }
  redraw = true;
  pixel_cnt += Math.floor(trans.data_length / 2);
}

/**
 * Returns (x,y) coordinate tuple of a 800x480 framebuffer
 */
function getCoords(addr: bigint) {
  if (start_address < 0) {
    return;
  }
  let loc_addr = addr - start_address;
  let y_start = loc_addr / divider;
  let x_start = (loc_addr - y_start * divider) / two;

  return { x: Number(x_start), y: Number(y_start) };
}

function isFB(module: string): boolean {
  return module.toLowerCase().indexOf("fb") != -1;
}
</script>

<template>
  <div class="box">
    <div class="columns">
      <div class="column">
        <h3 class="title is-3">Framebuffer</h3>
      </div>
      <div class="column is-narrow">
        <div class="tags has-addons">
          <span class="tag perf">{{ pps }}</span>
          <span class="tag">pixels/s</span>
        </div>
      </div>
    </div>
    <canvas ref="canvas" :width="w" :height="h"></canvas>
  </div>
</template>

<style scoped>
.perf {
  width: 60px;
}
</style>
