<script setup lang="ts">
import { onMounted, ref } from "vue";
import { drawLine, drawArrow } from "@/lib/tvm/architecture";
import type { Architecture, BusInterface, Line } from "@/lib/tvm/architecture";
import type { AppState, Transaction } from "@/types";
import EventBus from "@/lib/event_bus";

const props = defineProps<{ appState: AppState }>();

const fg = ref<HTMLCanvasElement>();
const bg = ref<HTMLCanvasElement>();
const observer = new ResizeObserver(resizeCallback);

const colors = {
  modules: "",
  modules_dark: "white",
  modules_bright: "rgb(200 200 200)",
  text: "black",
  bus: "rgb(128 128 128)",
  active: "green",
};
const font = "Ubuntu Sans Mono";
const bus_width = 8;

const core_args = {
  ysize: 0.15, // % of canvas height
  xsize: 0.1, // % of canvas width
};

const module_args = {
  left_border: 0.1, // % of canvas width
  top_margin: 0.1, // % of canvas height
};

const bus_module_line = 0.6; // length of the module bus line in % of module width

// current canvas size
let h = ref(750);
let w = ref(1000);

let frames_cnt = 0;
let arch: Architecture;
let cur_transaction: Transaction;
// indicates if the canvas should be redrawn during the next refresh
let redraw_fg = false;
let redraw_bg = false;

onMounted(() => {
  if (fg.value != undefined) {
    observer.observe(fg.value);
  }

  // First draw after init
  EventBus.on("TVM", () => {
    setTimeout(drawArchitecture, 10);
  });
  EventBus.on("trans", (trans) => {
    redraw_fg = true;
    cur_transaction = trans as Transaction;
  });

  setInterval(() => {
    if (frames_cnt > 0) {
      EventBus.emit("perf_fps", frames_cnt);
    }
    frames_cnt = 0;
  }, 1000);

  // adapt for dark or light mode
  colors.modules =
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? colors.modules_dark
      : colors.modules_bright;
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", (event) => {
      colors.modules = event.matches
        ? colors.modules_dark
        : colors.modules_bright;
    });

  loop();
});

function loop() {
  if (redraw_bg) {
    drawArchitecture();
    redraw_bg = false;
  }
  if (redraw_fg) {
    drawTransaction();
    frames_cnt++;
    redraw_fg = false;
  }
  requestAnimationFrame(loop);
}

function drawArchitecture() {
  if (bg.value == undefined || props.appState.modules == undefined) {
    return;
  }

  const ctx = bg.value.getContext("2d");
  if (ctx === null) {
    return;
  }

  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
  let numTrees = getNumTrees(w.value);
  ctx.textBaseline = "middle";
  ctx.textAlign = "center";

  const core_bus = drawCore(ctx);
  const module_bus = drawModules(
    ctx,
    props.appState.modules,
    props.appState.start_addrs,
    props.appState.end_addrs,
    core_bus,
    numTrees,
  );
  arch = drawBus(ctx, core_bus, module_bus, numTrees);
}

function drawTransaction() {
  if (fg.value == undefined || props.appState.modules == undefined) {
    return;
  }

  let ctx = fg.value.getContext("2d");
  if (ctx === null) {
    return;
  }
  ctx.clearRect(0, 0, w.value, h.value);

  let mod = arch.modules.find((x) => x.module_name === cur_transaction.target);
  if (mod === undefined) {
    return;
  }

  ctx.strokeStyle = colors.active;
  ctx.fillStyle = colors.active;
  ctx.lineWidth = bus_width;

  // horizontal module line
  drawLine(ctx, mod.line);
  // vertical tree line
  drawLine(
    ctx,
    mod.line.x2,
    mod.line.y2 + ctx.lineWidth / 2,
    mod.line.x2,
    arch.horizontal.y2 - ctx.lineWidth / 2,
  );
  // horizontal bus line
  drawLine(
    ctx,
    arch.core.line.x,
    arch.horizontal.y,
    mod.line.x2,
    arch.horizontal.y2,
  );
  // vertical core line
  drawLine(
    ctx,
    arch.core.line.x,
    arch.core.line.y,
    arch.core.line.x2,
    arch.core.line.y2 + ctx.lineWidth / 2,
  );

  drawArrow(ctx, mod, cur_transaction.action);

  // draw access width
  let x_a =
    mod.line.x2 < arch.core.line.x ? mod.line.x2 + 25 : mod.line.x2 - 25;
  let y_a = arch.horizontal.y;
  ctx.lineWidth = 5;
  drawLine(ctx, x_a - 10, y_a + 15, x_a + 10, y_a - 15);
  ctx.textAlign = "left";
  ctx.textBaseline = "bottom";
  ctx.font = setFontSize(24);
  ctx.fillText(
    cur_transaction.data_length.toString() +
      " [" +
      cur_transaction.data.toString(16) +
      "]",
    x_a - 5,
    y_a - 15,
  );

  // draw address above the horizontal module line
  ctx.font = setFontSize(16);
  ctx.textAlign = "left";
  ctx.fillText(
    cur_transaction.address.toString(16),
    mod.line.x2 + ctx.lineWidth,
    mod.line.y2 - ctx.lineWidth,
  );

  // simulation time
  ctx.textBaseline = "top";
  ctx.textAlign = "left";
  ctx.font = setFontSize(32);
  ctx.fillText(getSimTimeStr(cur_transaction.sim_time), 0, 0);

  // draw transaction number
  ctx.textAlign = "right";
  ctx.fillText(cur_transaction.trans_cnt.toString(), w.value, 0);
}

function drawCore(ctx: CanvasRenderingContext2D): BusInterface {
  ctx.fillStyle = colors.modules;
  const core_w = w.value * core_args.xsize;
  const core_h = h.value * core_args.ysize;
  const top_left_corner_x = w.value / 2 - core_w / 2;
  const top_left_corner_y = 0;
  ctx.fillRect(top_left_corner_x, top_left_corner_y, core_w, core_h);
  ctx.fillStyle = colors.text;
  ctx.font = setFontSize(48);
  ctx.fillText(
    "Core",
    top_left_corner_x + core_w / 2,
    top_left_corner_y + core_h / 2,
  );

  let pos_x = top_left_corner_x + core_w / 2;
  let pos_y = top_left_corner_y + core_h;
  return {
    module_name: "Core",
    line: {
      x: pos_x,
      y: pos_y,
      x2: pos_x,
      y2: pos_y + (h.value * module_args.top_margin + 25) / 2,
    },
  };
}

function drawModules(
  ctx: CanvasRenderingContext2D,
  modules: Array<string>,
  s_addrs: Array<string>,
  e_addrs: Array<string>,
  core: BusInterface,
  trees: number,
): Architecture {
  const bus_con = new Array<BusInterface>();
  const split_threshold = Math.floor(modules.length / trees) + 1;

  const top_y = core.line.y + h.value * module_args.top_margin;
  const w_module = w.value / (trees * 2);
  const h_module = (h.value - top_y) / (split_threshold + 2);
  let pos_x = w.value * module_args.left_border;
  let pos_y = top_y;
  let cur_tree = 1;

  for (let i = 1; i <= modules.length; i++) {
    ctx.fillStyle = colors.modules;
    ctx.fillRect(pos_x, pos_y, w_module, h_module);
    ctx.textBaseline = "middle";
    ctx.textAlign = "center";

    // name
    ctx.fillStyle = colors.text;
    ctx.font = setFontSize(36);
    ctx.fillText(
      modules[i - 1].toString(),
      pos_x + w_module / 2,
      pos_y + h_module / 2,
      w_module,
    );

    // addrs
    ctx.fillStyle = colors.bus;
    if (trees === 3) {
      ctx.font = setFontSize(16);
    } else {
      ctx.font = setFontSize(18);
    }
    ctx.textAlign = "right";
    ctx.textBaseline = "top";
    ctx.fillText(s_addrs[i - 1].toString().toUpperCase(), pos_x - 2, pos_y);
    ctx.textBaseline = "bottom";
    ctx.fillText(
      e_addrs[i - 1].toString().toUpperCase(),
      pos_x - 2,
      pos_y + h_module,
    );

    let line_y = pos_y + h_module / 2;
    bus_con.push({
      module_name: modules[i - 1].toString(),
      line: {
        x: pos_x,
        y: line_y,
        x2: pos_x - w_module * bus_module_line,
        y2: line_y,
      },
    });

    pos_y = pos_y + h_module + (h.value * module_args.top_margin) / 3;
    if (i % split_threshold === 0 && cur_tree != trees) {
      pos_x = pos_x + w_module * 2;
      pos_y = top_y;
      cur_tree++;
    }
  }
  return {
    modules: bus_con,
    core: core,
    verticals: new Array<Line>(),
    horizontal: { x: 0, x2: 0, y: core.line.y2, y2: core.line.y2 },
  };
}

// make text size configurable
function drawBus(
  ctx: CanvasRenderingContext2D,
  core: BusInterface,
  bus: Architecture,
  trees: number,
): Architecture {
  // bus attachment points
  ctx.fillStyle = colors.text;
  ctx.fillRect(core.line.x - 5, core.line.y - 5, 10, 10);
  bus.modules.forEach((mod) => {
    ctx.fillRect(mod.line.x - 5, mod.line.y - 5, 10, 10);
  });

  ctx.strokeStyle = colors.bus;
  ctx.lineWidth = bus_width;

  // vertical core line
  drawLine(
    ctx,
    core.line.x,
    core.line.y,
    core.line.x2,
    core.line.y2 + ctx.lineWidth / 2,
  );

  // horizontal module lines
  bus.modules.forEach((mod) => {
    drawLine(ctx, mod.line);
  });

  // vertical bus lines
  const mods_per_tree = Math.floor(bus.modules.length / trees) + 1;
  const tree_tops_x = new Array<number>();
  for (let i = 0; i < trees; i++) {
    let tree: Array<BusInterface>;
    if (i === trees - 1) {
      tree = bus.modules.slice(i * mods_per_tree, bus.modules.length);
    } else {
      tree = bus.modules.slice(
        i * mods_per_tree,
        i * mods_per_tree + mods_per_tree,
      );
    }
    const y_pos = tree.map((mod) => mod.line.y);

    if (tree.length === 0) {
      return bus;
    }

    tree_tops_x.push(tree[0].line.x2);

    drawLine(
      ctx,
      tree[0].line.x2,
      bus.core.line.y2,
      tree[0].line.x2,
      y_pos.reduce((a, b) => Math.max(a, b)) + ctx.lineWidth / 2,
    );
  }

  // horizontal connector line
  bus.horizontal.x =
    tree_tops_x.reduce((a, b) => Math.min(a, b)) - ctx.lineWidth / 2;
  bus.horizontal.x2 =
    tree_tops_x.reduce((a, b) => Math.max(a, b)) + ctx.lineWidth / 2;
  drawLine(ctx, bus.horizontal);
  return bus;
}

function getSimTimeStr(sim_time: bigint): string {
  let time_str;
  if (sim_time > 1000000) {
    let div = BigInt(1000000);
    let ns = sim_time % div;
    let ms = Math.floor(Number(sim_time / div));
    time_str = ms + " ms " + ns + " ns";
  } else {
    time_str = sim_time + " ns";
  }
  return time_str;
}

function getNumTrees(width: number): number {
  let numTrees = 3;
  if (width > 1000) {
    numTrees = 4;
  } else if (width > 1920) {
    numTrees = 5;
  }
  return numTrees;
}

function setFontSize(size: number): string {
  return Math.abs(size).toString() + "px " + font;
}

function resizeCallback(entries: ResizeObserverEntry[]) {
  const contentBox = entries[0].contentBoxSize[0];
  if (fg.value != undefined && contentBox != undefined) {
    w.value = Math.floor(contentBox.inlineSize);
    h.value = Math.floor(contentBox.blockSize);
    redraw_bg = true;
  }
}
</script>

<template>
  <div>
    <canvas id="bg" :width="w" :height="h" ref="bg"></canvas>
    <canvas id="fg" :width="w" :height="h" ref="fg"></canvas>
  </div>
</template>

<style scoped>
div {
  position: relative;
  height: 60vh;
}
canvas {
  height: 100%;
  width: 100%;
  background: transparent;
  position: absolute;
}
#fg {
  z-index: 2;
}
#bg {
  z-index: 1;
}
</style>
