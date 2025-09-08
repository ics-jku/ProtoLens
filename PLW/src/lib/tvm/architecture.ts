/*
 * Additional functionality needed by the Architecture.vue TVM
 */

export type BusInterface = {
  module_name: string;
  line: Line;
};

export type Line = {
  x: number;
  y: number;
  x2: number;
  y2: number;
};

export type Architecture = {
  core: BusInterface;
  modules: Array<BusInterface>;
  verticals: Array<Line>;
  horizontal: Line;
};

export function drawLine(ctx: CanvasRenderingContext2D, line: Line): void;
export function drawLine(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  x2: number,
  y2: number,
): void;
export function drawLine(
  ctx: CanvasRenderingContext2D,
  x: any,
  y?: number,
  x2?: number,
  y2?: number,
): void {
  ctx.beginPath();
  if (
    typeof x === "number" &&
    y != undefined &&
    x2 != undefined &&
    y2 != undefined
  ) {
    ctx.moveTo(x, y);
    ctx.lineTo(x2, y2);
  } else {
    ctx.moveTo(x.x, x.y);
    ctx.lineTo(x.x2, x.y2);
  }
  ctx.stroke();
  ctx.closePath();
}

export function drawArrow(
  ctx: CanvasRenderingContext2D,
  module: BusInterface,
  orientation: number,
) {
  // 1: Right arrow | 2: Left arrow
  if (orientation === 1) {
    ctx.beginPath();
    ctx.moveTo(module.line.x + 15, module.line.y);
    ctx.lineTo(module.line.x, module.line.y + 15);
    ctx.lineTo(module.line.x, module.line.y - 15);
    ctx.fill();
  } else {
    ctx.beginPath();
    ctx.moveTo(module.line.x - 5, module.line.y);
    ctx.lineTo(module.line.x + 15, module.line.y + 15);
    ctx.lineTo(module.line.x + 15, module.line.y - 15);
    ctx.fill();
  }
}
