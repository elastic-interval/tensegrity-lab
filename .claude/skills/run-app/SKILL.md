---
name: run-app
description: >
  Launch the native tensegrity-lab app on a fabric so Gerald can watch it build, stand, animate or
  take the wind. Builds release, starts the window in the background, confirms it is running, and
  lists the buttons. Invoke when the user says "run HH", "run Open Claw", "start the app", "show me
  the wind", "launch <fabric>", or invokes /run-app. Argument: a fabric name (defaults to Headless
  Hug).
---

# run-app

The app is a native wgpu window. It opens on Gerald's own screen, so he watches and drives it; you
launch it and tell him which buttons do what.

## Steps

1. **Build.** `cargo build --release` (seconds if nothing changed). Stop on a compile error.

2. **Launch in the background**, so the conversation goes on while the window is open. Use the
   Bash tool's `run_in_background`, with the output in the scratchpad:

   ```bash
   ./target/release/tensegrity_lab --fabric "Headless Hug" > <scratchpad>/app.log 2>&1
   ```

   Fabric names, exactly as `--fabric` takes them (`FabricName` in
   `src/build/dsl/fabric_library.rs`): `Headless Hug`, `Open Claw`, `Halo by Crane`,
   `Twisted Infinity`, `Propeller`, `Diamond`, `Quadruped`. With no `--fabric` it opens Open Claw;
   `--cycle` walks through them all. `--size medium` makes it about 1.5 m tall in the 2021
   Headless Hug's materials, on the wobble table; `--size small` is a model (see the
   `run-scale-model` skill). Gerald's own launcher is `scripts/lab.sh` (`scripts/lab.sh hh medium`).

3. **Confirm it started:** `pgrep -fl "tensegrity_lab --fabric"` shows the process, and
   `app.log` holds no panic. When the background task finishes, the window was closed (exit 0) or
   it crashed (read the log).

4. **Tell him the buttons that matter for what he asked**, from the table below. The buttons
   along the bottom are the controls, only those that apply showing;
   the one key is Esc, which steps back out of the parts view or a selection.

## Buttons

The fabric builds, pretensions, falls and settles by itself; most buttons come alive once it
stands (Viewing).

| Button | Does |
|---|---|
| Wind | Large only. Cycling: from the front, the side, behind, then off (`src/fabric/wind.rs`) |
| Quake / Stop quake | Medium only. Breeds DMX cues for the wobble table's stepper on copies of the piece, eight at a time out of sight, while the piece plays each find; it breeds on until pressed again (`src/fabric/quake.rs`, `src/fabric/cue.rs`) |
| Demo | While Quake runs. Plays the kept cues A–F in turn, 15 s each, breeding waiting, then breeds on |
| Take apart | Large only. Headless Hug and Open Claw: taken apart strut by strut on a crane hook, then straight away put together again, in real time (`src/build/assembly.rs`) |
| Start / Stop animation | Large only. The animation |
| Choose | In Viewing. A button for each fabric, to build it at the same size, and Back |
| Parts | Large only. The parts, full screen; Back to return |
| Deselect / Back to joint | Step back out of a selection |

The connector hardware always shows at real scale, and only at Large. Small and Medium start
building in fast-forward, Large never; there is no button for it. A movie is recorded from the command line
(`--record <seconds>`); `--white` shows it on white instead of the starry sky.

Click a joint or interval to select it; drag to orbit, wheel to zoom.

## Notes

- The settle takes 30 s of fabric time for Headless Hug, so it stands a little while after
  launching.
- Wind is rough on purpose: drag on every member from a 10 m/s breeze gusting to 15 every 8 s.
  The design wind loads are the engineer's.
- Web build instead: `trunk serve --release`, then open the address it prints. Without
  `--release` the WebAssembly is unoptimized and the physics runs at half speed.
