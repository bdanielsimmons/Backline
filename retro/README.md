# retro.dll

The classic Windows Media Player visualizers for Backline: Ryan Circelli's
[wmp-legacy-visualizers](https://github.com/ryancircelli/wmp-legacy-visualizers) (MIT, see
`LICENSE-wmp-legacy-visualizers.txt`), run in V8. Backline doesn't link this library: it downloads it from its site the
first time one is picked (`backline/src/ui/retro.rs`), and the release workflow builds it and puts it there.

- `src/lib.rs`: the C interface Backline calls (one thread).
- `retro.js`: the engine and `js/entry.ts` (Backline's bridge) in one script. Generated: don't edit.
- `presets.txt`: every preset the bundle has (family key, number, family, name), which Backline's menu reads. Generated
  with `retro.js`.
- `js/stubs/`: stand-ins for the two files of Goom's folder the rest imports. Goom's port is under the LGPL, so it's left
  out.

## Rebuilding the bundle

Made from the engine at commit `cc79d5f` (2026-11-05). With Node 24 and the TypeScript package (VS Code ships one in
`resources/app/extensions/node_modules/typescript`):

```
node js/bundle.mjs <typescript package folder> <wmp-legacy-visualizers>/visualizers/src retro.js presets.txt
```

Then bump `VERSION` in `src/lib.rs` and in `backline/src/ui/retro.rs` (a new version is a new file on the site and on
disk), and build: `cargo build --release` here.
