import type { Status } from "./types";

type Tone = "accent" | "warn" | "ok";

interface ProgressView {
  label: string;
  fraction: number | null;
  tone: Tone;
  phase: string;
  snap: boolean;
}

interface ProgressState {
  view: ProgressView | null;
  on: boolean;
  instantOff: boolean;
}

export const progress: ProgressState = $state({
  view: null,
  on: false,
  instantOff: false,
});

let lastRun: "scan" | "clean" = "scan";
const INDETERMINATE_WIDTH = 0.35;
let shownWidth = 0;

function describe(status: Status): string {
  if (status.phase === "done") return status.message;

  const verb = status.phase === "cleaning" ? "Cleaning" : "Scanning";
  const counter = status.total > 1 ? ` (${status.done + 1}/${status.total})` : "";
  return `${verb} · ${status.message}${counter}`;
}

export function begin(): void {
  progress.instantOff = true;
  progress.on = false;
  progress.view = null;
  shownWidth = 0;
}

export function report(status: Status): void {
  if (status.phase === "scanning") lastRun = "scan";
  if (status.phase === "cleaning") lastRun = "clean";

  const fraction =
    status.indeterminate || status.total === 0
      ? null
      : Math.min(1, Math.max(0.04, status.done / status.total));

  const snap = fraction !== null && fraction < shownWidth;
  shownWidth = fraction === null ? INDETERMINATE_WIDTH : fraction;

  progress.instantOff = false;
  progress.on = true;
  progress.view = {
    label: describe(status),
    fraction,
    tone:
      status.phase === "cleaning"
        ? "warn"
        : status.phase === "done" && lastRun === "clean"
          ? "ok"
          : "accent",
    phase: status.phase,
    snap,
  };
}

export function finish(): void {
  progress.instantOff = false;
  progress.on = false;
}