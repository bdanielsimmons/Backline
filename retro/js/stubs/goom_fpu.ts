// The three x87 helpers Shacojin takes from Goom's folder, written here over Plenoptic's FSIN/FCOS model (MIT),
// so nothing of Goom's (LGPL) is bundled.
import { sincos87 } from '../plenoptic/x87';

const SC = new Float64Array(4);

/** MSVC _ftol: truncate toward zero (out of range: 0x80000000) */
export function ftol(v: number): number {
  return v > -2147483649 && v < 2147483648 ? Math.trunc(v) : -2147483648;
}

/** FSIN's 64-bit result, rounded to a double by the next operation */
export function sin87(a: number): number {
  sincos87(a, SC);
  return SC[0] + SC[1];
}

/** FCOS's, the same way */
export function cos87(a: number): number {
  sincos87(a, SC);
  return SC[2] + SC[3];
}
