---
name: run-app
description: >
  Launch the native tensegrity-lab app on a fabric so Gerald can watch it build, stand, animate or
  take the wind. Builds release, starts the window in the background, confirms it is running, and
  lists the keys. Invoke when the user says "run HH", "run Open Claw", "start the app", "show me
  the wind", "launch <fabric>", or invokes /run-app. Argument: a fabric name (defaults to Headless
  Hug).
---

# run-app

The app is a native wgpu window. It opens on Gerald's own screen, so he watches and drives it; you
launch it and tell him which keys do what.

## Steps

1. **Build.** `cargo build --release` (seconds if nothing changed). Stop on a compile error.

2. **Launch in the background**, so the conversation goes on while the window is open. Use the
   Bash tool's `run_in_background`, with the output in the scratchpad:

   ```bash
   ./target/release/tensegrity_lab --fabric "Headless Hug" > <scratchpad>/app.log 2>&1
   ```

   Fabric names, exactly as `--fabric` takes them (`FabricName` in
   `src/build/dsl/fabric_library.rs`): `Headless Hug`, `Open Claw`, `Halo by Crane`,
   `Twisted Infinity`, `Propeller`, `Diamond`. With no `--fabric` it opens Open Claw;
   `--cycle` walks through them all.

3. **Confirm it started:** `pgrep -fl "tensegrity_lab --fabric"` shows the process, and
   `app.log` holds no panic. When the background task finishes, the window was closed (exit 0) or
   it crashed (read the log).

4. **Tell him the keys that matter for what he asked**, from the table below. The window also
   lists the keys available at each moment.

## Keys

The fabric builds, pretensions, falls and settles by itself; most keys work once it stands
(Viewing).

| Key | Does |
|---|---|
| W | Wind, cycling: from the front, the side, behind, then off (`src/fabric/wind.rs`) |
| A | Start / stop the animation |
| F / f | Faster / slower animation |
| T / t | Faster / slower time |
| C | Connector hardware (Open Claw, Headless Hug), or colour by role |
| P | The parts, full screen; P or Esc to return |
| S | Movement stats |
| G | Live GPU physics |
| X | Physics testing (gravity) |
| M / m, R / r | Mass and rigidity up / down, while physics testing |
| J | Jump |
| Enter | Fast-forward the build, or reload the fabric |
| N | Snapshot |
| M | Record a movie (outside physics testing) |
| Esc | Cancel the selection, then exit |

Click a joint or interval to select it; drag to orbit, wheel to zoom.

## Notes

- The settle takes 30 s of fabric time for Headless Hug, so it stands a little while after
  launching.
- Wind is rough on purpose: drag on every member from a 10 m/s breeze gusting to 15 every 8 s.
  The design wind loads are the engineer's.
- Web build instead: `trunk serve`, then open the address it prints.
