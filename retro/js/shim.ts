// The few browser globals the engine touches, for a bare JavaScript engine with no page around it.
const g = globalThis as any;

class NoCanvas {
  width = 16;
  height = 16;
  getContext(): null {
    return null;
  }
}
if (!g.OffscreenCanvas) g.OffscreenCanvas = NoCanvas;

if (!g.atob) {
  const A = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  const map = new Int16Array(128).fill(-1);
  for (let i = 0; i < 64; i++) map[A.charCodeAt(i)] = i;
  g.atob = (s: string): string => {
    let out = '', bits = 0, acc = 0;
    for (let i = 0; i < s.length; i++) {
      const v = map[s.charCodeAt(i) & 127];
      if (v < 0) continue;
      acc = (acc << 6) | v;
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        out += String.fromCharCode((acc >> bits) & 255);
      }
    }
    return out;
  };
}

export { NoCanvas };
