# sample-canvas

Sample apps that dogfood [Div](https://github.com/build-with-div/div) by
porting popular npm canvas packages to Rust:

- Konva and rough.js, on the `<canvas>` 2D context;
- three.js's WebGPU renderer, on the `<canvas>` WebGPU context.

Each port reports the web platform features it needs. Div's job is to
implement those features, matching Chromium's behavior and pixels, not to
offer workarounds.

Any temporary scaffolding in a sample is labeled `// div-gap: W…` with the
platform feature that replaces it, and logged in
[docs/framework-gaps.md](docs/framework-gaps.md).

Phase 1 was design only:

- [docs/design.md](docs/design.md): packages, then the platform features
  they use (W1–W21), what Div has, owners and sizes, architecture, the
  fidelity plan and the planned ports.
- [docs/rfc-canvas.md](docs/rfc-canvas.md): the proposed Div API for the
  canvas element and its 2D and WebGPU contexts, as an RFC draft.
- [docs/npm-platform-usage.md](docs/npm-platform-usage.md): which platform
  APIs each package's published code calls.
- [docs/wpt-canvas-census.md](docs/wpt-canvas-census.md): an estimate of
  which WPT `html/canvas` tests a Div 2D context could port.
- [docs/probes](docs/probes): probes of Div's public API at the pinned
  commit.

Phase 2, porting the packages, follows [docs/phase2-plan.md](docs/phase2-plan.md).

## Samples

- [samples/roughjs](samples/roughjs): a port of rough.js 4.6.6's generator,
  compared op by op with rough.js itself (milestone P2-1).

## Build

The repository is one Cargo workspace. Div is pinned to one commit in
`docs/probes/Cargo.toml` and in the patch table of the root `Cargo.toml`.
Div's Skia is built from source with Div's CI profile
(`.cargo/config.toml`).

On Windows:

```
. tools\win-build.ps1
cargo test --release -- --nocapture --test-threads 1
cargo run --release -p canvas-probes --bin frame_loop -- 3
```

On Linux, install Div's build dependencies (listed in
`.github/workflows/ci.yml`) and run the same cargo commands.

To re-record the rough.js oracle (Node 22):

```
python -I tools/rough-oracle/make_cases.py
cd tools/rough-oracle
npm ci
node oracle.cjs
```
