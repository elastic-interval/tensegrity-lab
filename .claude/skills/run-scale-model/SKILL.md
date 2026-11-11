---
name: run-scale-model
description: >
  Launch the native app showing a fabric at scale-model size, for building Gerald's model: every
  length shown in model millimetres, and each cable with the caliper reading to cut its cord to.
  Invoke when the user says "launch for the scale model", "model lengths", "build the model",
  "caliper readings", or invokes /run-scale-model. Arguments: a fabric name (defaults to Headless
  Hug) and the model's scale (defaults to 1:10).
---

# run-scale-model

Gerald builds a scale model before each big build, as a reference to hold while building the real
one: brass tube struts, dyneema cords tied to rings, M4 nuts. Open Claw's was about 1:10 (73 cm);
Headless Hug's is planned at about 1:10 too. Assume 1:10 unless he says otherwise.

## The scale is a multiplier

`--model-scale` multiplies every length shown, so **a 1:10 model is `--model-scale 0.1`**. The
app then labels it "(10:1)". `--model-scale 10` would show everything ten times too big. It only
changes what is displayed; the simulation runs at full size as always.

## Steps

1. **Build:** `cargo build --release`.

2. **Launch in the background** with the Bash tool's `run_in_background`, output to the
   scratchpad:

   ```bash
   ./target/release/tensegrity_lab --fabric "Headless Hug" --model-scale 0.1 > <scratchpad>/app.log 2>&1
   ```

   Fabric names as in the `run-app` skill. For another scale, 1:N is `--model-scale` 1/N
   (1:12 is `0.0833`).

3. **Confirm it started:** `pgrep -fl "tensegrity_lab --fabric"`, and no panic in `app.log`.

4. **Tell him how to read it**, below.

## Reading the model's lengths

Let it build and stand, then click:

- **A cable or strut:** its distance in model millimetres, joint to joint. A cable also shows
  **Caliper**: the reading to set on his caliper, which holds the cord almost folded in half.
- **A joint:** its height and, if it rests on the ground, where (model scale).
- The stats show the fabric "Built at" with the scale, e.g. "(10:1)".

What the lengths are:

- **Joint to joint**, the figure as it stands in the app: right for a model whose cords tie to
  rings, without the real connectors' pins and terminals.
- The real build's lengths, pin to pin and unloaded, are in the handoff JSON, not here.

Mind the caliper's range: its calibration (`src/caliper.rs`) covers cords of 29 to 140 mm.
Longer ones are extrapolated, and anything shorter reads as 15 mm. At 1:10 Headless Hug's cables
run from about 28 to 174 mm, so check the longest and shortest by measuring.

In model mode the connector (C) and parts (P) views are off; the other keys work as in `run-app`.
