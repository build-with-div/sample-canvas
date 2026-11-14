# three-shell

A three.js example shell on Div (milestone P2-6b):

- a toolbar: Reset, Auto-rotate, Damping, Settings, and a status line;
- an objects pane, whose rows move the camera's target;
- a splitter that resizes the pane and the viewport;
- a viewport `<canvas>` that OrbitControls drives with the mouse, the wheel
  and the arrow keys;
- a settings popover over the canvas (rotate speed, screen-space panning).

The camera and its controls are the three.js port in
[samples/three](../three). The viewport's size sets the camera's aspect and
the canvas's size, as a three.js example's resize handler does.

The canvas shows nothing yet: Div has no WebGPU canvas (W17), so the scene
is milestone P2-6c.

```
cargo run -p three-shell
```

## Fidelity

`tests/fidelity_shell.rs` replays scripted input on the shell and compares
it with the same page on three.js 0.187.0 in Chrome for Testing
147.0.7727.50 (`tools/three-reference`), after load and after every step.
It compares every element's box, attributes and text, the focused element,
the camera, the controls, and the controls' events. See
[docs/phase2-plan.md](../../docs/phase2-plan.md) (P2-6b) for the
measurements, and `tests/fixtures/fidelity/shell-div-differences.txt` for
the known differences and the gaps that own them.

```
cargo test -p three-shell --test fidelity_shell -- --nocapture
```

To recapture the reference, with `CHROME` the `chrome` binary of Chrome for
Testing 147.0.7727.50 and three.js installed by
`npm ci --prefix tools/three-oracle`:

```
node tools/three-reference/capture.mjs CHROME samples/three-shell/tests/fixtures/fidelity/shell-chromium.json
```

Scaffolding is labeled `// div-gap:` with the platform feature that
replaces it; [docs/framework-gaps.md](../../docs/framework-gaps.md) lists
each one.
