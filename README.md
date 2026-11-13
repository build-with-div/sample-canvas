# sample-canvas

Sample apps that dogfood [Div](https://github.com/build-with-div/div) for
canvas work. They take both routes:

- **A, surface handoff:** the app renders into an element-sized surface.
- **B, a built-in 2D context:** like `CanvasRenderingContext2D`, on Div's
  Skia.

The samples use only Div's public API. Wherever Div lacks something, the
workaround is labeled `// div-gap:`, and the gap is recorded in
[docs/framework-gaps.md](docs/framework-gaps.md).

Phase 1 is design only:

- [docs/design.md](docs/design.md): what Div has today, the recommended
  design for both routes, owners and sizes, risks, the fidelity plan and the
  planned samples.
- [docs/rfc-canvas.md](docs/rfc-canvas.md): the proposed Div API, as an RFC
  draft for the Div tracks.
- [docs/probes](docs/probes): probes of the public API at the pinned Div
  commit.
- [docs/wpt-canvas-census.md](docs/wpt-canvas-census.md): an estimate of
  which WPT `html/canvas` tests a Div 2D context could port.

## Build

Div is pinned to one commit in `docs/probes/Cargo.toml`. Its Skia is built
from source with Div's CI profile (`.cargo/config.toml`).

On Windows:

```
. tools\win-build.ps1
cd docs\probes
cargo test --release -- --nocapture --test-threads 1
cargo run --release --bin frame_loop -- 3
```

On Linux, install Div's build dependencies (listed in
`.github/workflows/ci.yml`) and run the same cargo commands.
