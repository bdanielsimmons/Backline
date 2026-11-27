// Bundles the visualizer engine (wmp-legacy-visualizers, MIT, Ryan Circelli) and Backline's bridge (entry.ts) into
// one classic script for an embedded JavaScript engine, and lists its presets for Backline's menu. Each TypeScript
// file is compiled to CommonJS by the TypeScript compiler and wrapped in a small module table. Goom's folder (LGPL)
// is left out: stubs/ stands in for the two files the rest imports.
//
//   node bundle.mjs <typescript package dir> <engine's visualizers/src> <out.js> <presets.txt>
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const here = path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
const [tsDir, engineSrc, out, presetsOut] = process.argv.slice(2);
const ts = createRequire(import.meta.url)(path.resolve(tsDir, 'lib', 'typescript.js'));
const OPTS = {
  target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS, useDefineForClassFields: false,
  isolatedModules: true, importHelpers: false, removeComments: true, sourceMap: false,
};
const STUBS = { 'goom/index.ts': 'stubs/goom_index.ts', 'goom/fpu.ts': 'stubs/goom_fpu.ts' };
const rel = (file) => path.relative(engineSrc, file).replace(/\\/g, '/');

function resolve(spec, from) {
  const base = spec.startsWith('V/') ? path.join(engineSrc, spec.slice(2))
    : spec.startsWith('.') ? path.resolve(path.dirname(from), spec) : null;
  if (!base) throw new Error(`unknown import ${spec} in ${from}`);
  if (base.endsWith('?raw') && fs.existsSync(base.slice(0, -4))) return base;
  for (const p of [base + '.ts', path.join(base, 'index.ts'), base]) if (fs.existsSync(p) && fs.statSync(p).isFile()) return p;
  throw new Error(`cannot find ${spec} from ${from}`);
}

const ids = new Map(), parts = [];
function add(file) {
  if (ids.has(file)) return ids.get(file);
  const id = ids.size;
  ids.set(file, id);
  if (file.endsWith('?raw')) {   // a bundler's "?raw" import: the file's text as the default export
    parts.push(`${id}: function (module, exports) { exports.__esModule = true; exports.default = ${JSON.stringify(fs.readFileSync(file.slice(0, -4), 'utf8'))}; }`);
    return id;
  }
  const r = rel(file);
  if (r.startsWith('goom/') && !STUBS[r]) throw new Error(`${r} would bring Goom's code in`);
  const text = fs.readFileSync(STUBS[r] ? path.join(here, STUBS[r]) : file, 'utf8');
  const res = ts.transpileModule(text, { compilerOptions: OPTS, fileName: file, reportDiagnostics: true });
  const bad = (res.diagnostics || []).filter((d) => d.category === ts.DiagnosticCategory.Error);
  if (bad.length) throw new Error(file + ': ' + ts.flattenDiagnosticMessageText(bad[0].messageText, '\n'));
  const slot = parts.push('') - 1;
  const code = res.outputText.replace(/require\("([^"]+)"\)/g, (_, spec) => `__req(${add(resolve(spec, file))})`);
  parts[slot] = `/* ${r} */ ${id}: function (module, exports, __req) {\n${code}\n}`;
  return id;
}

add(path.join(here, 'entry.ts'));
const table = [...ids.values()].map((id) => parts[id]).join(',\n');
const js = `(function () {
var __defs = {\n${table}\n}, __cache = {};
function __req(id) {
  var m = __cache[id];
  if (m) return m.exports;
  m = __cache[id] = { exports: {} };
  __defs[id].call(m.exports, m, m.exports, __req);
  return m.exports;
}
__req(0);
})();
`;
fs.writeFileSync(out, js);
console.log(`${ids.size} modules, ${(js.length / 1048576).toFixed(2)} MB`);

// the preset list, as the bundle itself gives it
globalThis.performance ??= { now: () => Date.now() };
await import(pathToFileURL(path.resolve(out)).href);
const list = globalThis.BL.presets();
fs.writeFileSync(presetsOut, list + '\n');
console.log(`${list.split('\n').length} presets`);
