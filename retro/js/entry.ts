// Backline's bridge to wmp-legacy-visualizers (MIT, Ryan Circelli): one visualizer at a time, fed interleaved
// stereo float32 through WMP's own analyzer, drawn into a buffer Backline owns. Called by ../src/lib.rs.
import { NoCanvas } from './shim';
import { createEngine, makeLevel, pictureFromRGBA, PRESETS, samplingOf, type Picture, type VisEngine, type VisKind } from 'V/index';
import { createPcmSource } from 'V/audio/pcm';
import { versionsOf } from 'V/catalog';

const level = makeLevel();
const pcm = createPcmSource(48000);
const canvas = new NoCanvas();
let eng: VisEngine | null = null;
let kind: VisKind = 'alchemy';
let out = new Uint32Array(0);
let inp = new Float32Array(0);
let pics: Picture[] = [];

/** Whose analyzer arithmetic feeds a preset: the newest WMP that drew it (as Ryan's player does). */
function build(k: VisKind, preset: number): void {
  const v = versionsOf(k, preset)[1];
  pcm.setBuild(v <= 8 ? 'wmp7' : v <= 10 ? 'wmp9' : v === 11 ? 'wmp11' : 'wmp12', k === 'musical');
}

(globalThis as any).BL = {
  /** every preset in WMP's order, one per line: vis, preset, group, name (tab-separated) */
  presets: (): string => PRESETS.map((p) => [p.vis, p.preset, p.group, p.name].join('\t')).join('\n'),
  /** the frame (0x00RRGGBB) and sample buffers, owned by Backline */
  buffers(o: ArrayBuffer, i: ArrayBuffer): void {
    out = new Uint32Array(o);
    inp = new Float32Array(i);
  },
  /** a visualizer of its own (a family change: WMP makes a fresh object) */
  open(k: VisKind, preset: number, w: number, h: number): void {
    kind = k;
    canvas.width = w;
    canvas.height = h;
    eng = createEngine(k, canvas as unknown as HTMLCanvasElement, { preset, scale: 'auto' });
    if (pics.length) eng.setPictures(pics);
    build(k, preset);
  },
  /** another preset of the same family (its state kept, as WMP's SetCurrentPreset) */
  preset(n: number): void {
    if (!eng) return;
    eng.setPreset(n);
    build(kind, eng.preset);
  },
  /** the view's size in pixels; the engine draws at its own size within it */
  size(w: number, h: number): void {
    eng?.resize(w, h);
  },
  rate(r: number): void {
    pcm.setRate(r);
  },
  /** n interleaved stereo floats from the sample buffer */
  push(n: number): void {
    if (n > 0) pcm.push(inp.subarray(0, n));
  },
  /** One WMP Render(): state 2 playing, 1 paused, 0 stopped. Returns w | h << 16, or 0 for no frame. */
  render(state: number): number {
    if (!eng) return 0;
    pcm.fill(level);
    level.state = state;
    const s = eng.render(level);
    if (!s || s.w * s.h > out.length) return 0;
    out.set(s.px.length === s.w * s.h ? s.px : s.px.subarray(0, s.w * s.h));
    return s.w | (s.h << 16);
  },
  /** 1 when this preset's frame is stretched smoothly (WMP 12's blits), 0 for nearest (pixels kept sharp) */
  smooth(): number {
    return eng && samplingOf(kind, eng.preset) === 'smooth' ? 1 : 0;
  },
  /** Picture Viz's photo: the song's cover (RGBA) */
  picture(rgba: ArrayBuffer, w: number, h: number): void {
    pics = w > 0 && h > 0 ? [pictureFromRGBA(w, h, new Uint8Array(rgba))] : [];
    eng?.setPictures(pics);
  },
};
