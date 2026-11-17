export const HOLD_DELAY_MS = 380;
export const HOLD_LOCK_DISTANCE = 64;
/** 长按加速档位：右滑升档、左滑降档，最低 2x。 */
export const HOLD_SPEED_STEPS = [2, 3, 4, 6];
/** 每累计横向滑动 80px 换一档（增量累计，来回滑可升可降）。 */
export const HOLD_STEP_DISTANCE = 80;
export type HoldZone = "fast" | "slow" | "menu";
export type HoldMenuBounds = { left: number; top: number; size: number };
/** 一次长按内的换挡状态：当前档位 + 增量滑动余量。 */
export type HoldSpeed = { rate: number; lastX: number; travel: number };

export function createHoldSpeed(x: number): HoldSpeed {
  return { rate: HOLD_SPEED_STEPS[0], lastX: x, travel: 0 };
}

/** 舞台坐标：CSS rotate(90deg) 时屏幕 dy=舞台 dx、屏幕 dx=-舞台 dy，其余情况原样。 */
export function stageXY(rotated: boolean, dx: number, dy: number): [number, number] {
  return rotated ? [dy, -dx] : [dx, dy];
}

/** 累计横向增量；跨过一档时更新档位并震动一次，返回新档位，否则返回 null。 */
export function shiftHoldSpeed(s: HoldSpeed, x: number): number | null {
  if (!Number.isFinite(x)) return null;
  s.travel += x - s.lastX;
  s.lastX = x;
  const steps = Math.trunc(s.travel / HOLD_STEP_DISTANCE);
  if (!steps) return null;
  s.travel -= steps * HOLD_STEP_DISTANCE;
  const i = HOLD_SPEED_STEPS.indexOf(s.rate);
  const next = HOLD_SPEED_STEPS[Math.min(HOLD_SPEED_STEPS.length - 1, Math.max(0, i + steps))];
  if (next === s.rate) { s.travel = 0; return null; }
  s.rate = next;
  try { navigator.vibrate?.(15); } catch { /* ignore */ }
  return next;
}

const dimension = (value: number) => Number.isFinite(value) && value > 0 ? value : 0;

/** CSS-pixel square shared by hit testing and the first-visit guide. */
export function getHoldMenuBounds(width: number, height: number): HoldMenuBounds {
  const w = dimension(width), h = dimension(height);
  const short = Math.min(w, h);
  const size = Math.min(short, 112, Math.max(80, short * 0.26));
  return { left: (w - size) / 2, top: (h - size) / 2, size };
}

/** x/y are relative to the visible touch rectangle, including in landscape. */
export function getHoldZone(x: number, y: number, width: number, height: number): HoldZone {
  const w = dimension(width), h = dimension(height);
  if (!w || !h || !Number.isFinite(x) || !Number.isFinite(y)) return "menu";
  const { left, top, size } = getHoldMenuBounds(w, h);
  if (x >= left && x <= left + size && y >= top && y <= top + size) return "menu";
  return x / w - y / h >= 0 ? "fast" : "slow";
}
