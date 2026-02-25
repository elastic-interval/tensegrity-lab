# Mockup: Physical Build Cost and Weight Reference

The Mockup is a physical tensegrity structure built from the design defined in the codebase as `Mockup` (see `fabric_library.rs`). It consists of a `SingleTwistLeft` seed brick plus a column of 2, yielding 3 single-twist bricks at a scale of 0.59m per fabric unit.

This document records the actual costs and weights from the physical build, providing per-component reference values for estimating new designs.

## Structure Summary

| Count | Component | Description |
|------:|-----------|-------------|
| 9 | Struts | Telescoping aluminum tubes (compression) |
| 36 | Cables | Dyneema rope with quick links (tension) |
| 18 | Connector heads | Steel hinge assemblies, one per strut end |
| 1 | Tensioning clamp | Reusable tool for pretensioning cables |

## Strut Assembly

Each strut is a telescoping assembly with three sections:

```
[Head]--[Inner tube]====[Outer tube]====[Inner tube]--[Head]
```

- **Outer tube**: Aluminum 60x4mm (60mm OD, 4mm wall), fixed-length compression member
- **Inner tubes**: Aluminum 50x4mm (50mm OD, 4mm wall), slide inside the outer tube, secured with lock pins
- **Lock pins**: 2 per inner tube, prevent the inner tube from sliding out

For the Mockup at scale 0.59m:
- Outer tube length: ~1.09m (volume 764,920 mm3)
- Inner tube length: ~0.65m each (volume 378,524 mm3)

### Strut Body

| Part | Material | Qty/strut | Weight (kg) | Cost (€) |
|------|----------|----------:|------------:|---------:|
| Outer tube | Aluminum 60x4 | 1 | 2.07 | 31.18 |
| Inner tube | Aluminum 50x4 | 2 | 1.02 | 21.68 |
| Lock pin | Steel | 4 | 0.05 | 0.81 |
| **Per strut body** | | | **4.31** | **77.78** |

## Connector Head

Each strut end has a connector head: a steel assembly that provides cable attachment points. The head consists of stacked connector discs at different angles, separated by POM plastic spacers, all held together by an M20 bolt.

This matches the `HingeDimensions` in the codebase: disc_thickness=10mm, disc_separator_thickness=3mm, cap_thickness=6mm.

### Head Components

| Part | Material | Qty/head | Weight (kg) | Cost (€) |
|------|----------|----------:|------------:|---------:|
| Tube stub | S235 steel 60x4 | 1 | 0.28 | 8.45 |
| Tapped ring M20 | S235 steel, 10mm | 1 | 0.20 | 6.07 |
| 30-degree disc | S235 steel, 10mm | 2 | 0.34 | 6.15 |
| 60-degree disc | S235 steel, 10mm | 2 | 0.34 | 5.98 |
| POM separator | POM plastic, 3mm | 5 | 0.01 | 1.17 |
| Fender washer | M20 | 1 | 0.04 | 0.43 |
| Lock nut | M20 | 1 | 0.06 | 0.64 |
| Bolt | M20x100 | 1 | 0.29 | 3.07 |
| Welding | 0.17 hrs @ €65/hr | | -- | 11.05 |
| Coating | Alu-zinc spray | 1 | -- | 1.67 |
| Coating labor | 0.12 hrs @ €65/hr | | -- | 7.80 |
| **Per head (material)** | | | **2.28** | **€69.30** |
| Assembly labor | 0.10 hrs @ €65/hr | | | 6.50 |
| **Per head (total)** | | | **2.28** | **€75.80** |

Each head has **4 connector discs** (2 at 30 degrees, 2 at 60 degrees), providing 4 cable attachment slots. With 18 heads x 4 slots = 72 slots, matching exactly 36 cables x 2 endpoints = 72.

The disc angles (30 and 60 degrees) correspond to the `HingeBend` angles in the codebase. Each disc can be installed in either orientation, giving effective angles of +/-30 and +/-60 degrees.

## Cable Assembly

Each cable is a length of Dyneema rope with a quick link spliced at each end.

| Part | Specification | Qty/cable | Weight (kg) | Cost (€) |
|------|---------------|----------:|------------:|---------:|
| Dyneema rope | 14mm, ~1.02m | 1 | 0.11 | 10.78 |
| Quick link | 12B | 2 | 0.29 | 2.65 |
| **Per cable (material)** | | | **0.69** | **€16.08** |
| Splicing labor | 0.44 hrs @ €65/hr | | | 28.89 |
| **Per cable (total)** | | | **0.69** | **€44.97** |

Cable material weight: Dyneema at 11 kg per 100m; quick links at 0.29 kg each (steel).

Dyneema unit price: **€10.57/m** (14mm diameter).

## Complete Strut (Body + 2 Heads)

| Component | Weight (kg) | Material (€) | Labor (€) |
|-----------|------------:|-------------:|----------:|
| Strut body | 4.31 | 77.78 | -- |
| 2 connector heads | 4.56 | 138.60 | 13.00 |
| **Per strut total** | **8.87** | **€216.38** | **€13.00** |

## Tensioning Clamp (Reusable Tool)

A tensioning clamp is used during construction to pretension cables. It is a reusable tool, not a structural component. One clamp is needed regardless of structure size.

| Part | Qty | Cost (€) |
|------|----:|---------:|
| Bottom plate (S235) | 2 | 38.71 |
| Middle plate (S235) | 2 | 46.17 |
| Top plate (S235) | 2 | 39.67 |
| Strip (S235) | 2 | 63.10 |
| Carriage bolt + nut (galvanized) | 3 | 3.30 |
| 3D printed ring | 2 | 20.00 |
| Bottle jack (2 ton) | 2 | 55.88 |
| Welding (8 hrs @ €75/hr) | | 600.00 |
| **Total** | | **€866.84** |

## Other Costs

| Item | Cost (€) |
|------|---------:|
| Shipping | 120.00 |
| Cable setup jig | 200.00 |
| **Total** | **€320.00** |

## Mockup Totals

### Weight: 104.7 kg

| Component | Units | Per unit (kg) | Total (kg) |
|-----------|------:|--------------:|-----------:|
| Outer tubes | 9 | 2.07 | 18.6 |
| Inner tubes | 18 | 1.12 | 20.1 |
| Connector heads | 18 | 2.28 | 41.1 |
| Cables | 36 | 0.69 | 24.9 |
| **Structure** | | | **104.7** |

### Cost: €4,870.29 (excl. VAT)

| Category | Material (€) | Labor (€) | Total (€) |
|----------|-----------:|--------:|--------:|
| Strut bodies (outer + inner) | 700.11 | -- | 700.11 |
| Connector heads (18) | 1,247.40 | 117.00 | 1,364.40 |
| Cables (36) | 578.94 | 1,040.00 | 1,618.94 |
| Tensioning clamp | 866.84 | -- | 866.84 |
| Other | 320.00 | -- | 320.00 |
| **Total** | **€3,713.29** | **€1,157.00** | **€4,870.29** |

All amounts in euros, excluding VAT. Labor rate: €65/hr (general assembly), €75/hr (welding).

## Estimation Guide

To estimate the cost and weight of a different design using these materials:

### Step 1: Count struts and cables

Run the design in the simulation and count push intervals (struts) and pull intervals (cables, including all pull-like roles: Pulling, Circumference, BowTie, PrismPull).

### Step 2: Determine scale and lengths

The simulation reports interval ideal lengths in millimeters (already scaled). Use these directly — they are the physical lengths of the struts and cables.

### Step 3: Calculate tube costs

Tube material costs scale linearly with strut length. The Mockup's strut body costs €77.78 for an average push interval of 1,419mm, giving a rate of **€0.055/mm** of strut.

Connector head costs are **fixed per head** at €69.30 material + €6.50 labor (for 4 discs). They do not depend on strut length.

### Step 4: Calculate cable costs

- Dyneema: €10.57/m of cable length
- Quick links: 2 × €2.65 = €5.30 per cable (fixed)
- Splicing labor: €28.89 per cable (fixed)

### Step 5: Count connector discs per head

The Mockup uses 4 discs per head. If a strut end has fewer cables, it needs fewer discs. Each disc adds approximately:
- Weight: 0.34 kg
- Cost: ~€6.00 (material)
- 1 additional POM separator: 0.01 kg, €1.17

A head with 3 discs costs approximately €62.06 instead of €69.30.

### Material Specifications Reference

| Material | Specification | Density |
|----------|---------------|---------|
| Outer tube | Aluminum 6063-T6, 60x4mm | 2,700 kg/m3 |
| Inner tube | Aluminum 6063-T6, 50x4mm | 2,700 kg/m3 |
| Connector parts | S235 structural steel | 7,850 kg/m3 |
| Separators | POM (polyoxymethylene) | 1,420 kg/m3 |
| Cable | Dyneema SK75, 14mm | 11 kg/100m |
| Quick links | Steel, size 12B | 0.29 kg each |
| Bolt | M20x100, grade 8.8 | 0.29 kg |

---

## OpenClaw Estimate

The OpenClaw design (see `fabric_library.rs`) is an omni-symmetrical seed with three legs of 3 bricks each, prisms on three end faces and the top face, and an open bottom face.

### Simulation Data (default scale 1.5m)

Obtained by running the build to completion and counting intervals:

| | Mockup (0.59m) | OpenClaw (1.5m) | Ratio |
|-|---------------:|----------------:|------:|
| **Struts** | 9 | 46 | 5.1× |
| **Cables** | 36 | 180 | 5.0× |
| **Joints** | 24 | 92 | 3.8× |
| **Strut ends** | 18 | 92 | 5.1× |

Cable breakdown by role:

| Role | Count |
|------|------:|
| Pulling | 39 |
| Circumference | 72 |
| BowTie | 45 |
| PrismPull | 24 |
| **Total cables** | **180** |

### Strut Lengths

| Length group | Count |
|-------------|------:|
| ~2,100mm | 3 |
| ~2,340mm | 9 |
| ~2,720mm | 9 |
| ~3,200mm | 9 |
| ~3,750mm | 10 |
| ~4,500mm | 3 |
| ~4,860mm | 3 |
| **Average** | **3,182mm** |

Range: 2,100mm to 4,860mm. Total strut tube material: 146.4m.

### Cable Lengths

Average cable: 1,701mm. Range: 923mm to 3,295mm. Total Dyneema: 306.2m.

### Cables Per Strut End (Disc Requirement)

| Cables | Strut ends | Discs/head |
|-------:|-----------:|-----------:|
| 3 | 8 | 3 |
| 4 | 84 | 4 |

Total connector discs needed: 84 × 4 + 8 × 3 = **360** (vs 72 in Mockup).

### Cost Estimate at Default Scale (1.5m)

Strut tube costs scale by strut length at €0.055/mm. Connector heads and cable hardware are fixed per unit.

**Strut bodies (46 struts):**

| | Calculation | Total (€) |
|-|-------------|----------:|
| Tube material | 146,390mm total × €0.055/mm | 8,051 |
| **Strut bodies** | | **€8,051** |

**Connector heads (92 strut ends):**

| | Calculation | Total (€) |
|-|-------------|----------:|
| 84 heads × 4 discs | 84 × €69.30 | 5,821 |
| 8 heads × 3 discs | 8 × €62.06 | 497 |
| Assembly labor | 92 × €6.50 | 598 |
| **Connector heads** | | **€6,916** |

**Cables (180 cables):**

| | Calculation | Total (€) |
|-|-------------|----------:|
| Dyneema | 306.2m × €10.57/m | 3,237 |
| Quick links | 360 × €2.65 | 954 |
| Splicing labor | 180 × €28.89 | 5,200 |
| **Cables** | | **€9,391** |

**Fixed costs:**

| Item | Cost (€) |
|------|---------:|
| Tensioning clamp (reusable) | 867 |
| Shipping (estimate) | 300 |
| Cable jig | 200 |
| **Fixed** | **€1,367** |

### OpenClaw Cost Summary (1.5m scale)

| Category | Material (€) | Labor (€) | Total (€) |
|----------|-----------:|--------:|--------:|
| Strut bodies | 8,051 | -- | 8,051 |
| Connector heads (92) | 6,318 | 598 | 6,916 |
| Cables (180) | 4,191 | 5,200 | 9,391 |
| Fixed costs | 1,367 | -- | 1,367 |
| **Total** | **€19,927** | **€5,798** | **€25,725** |

### OpenClaw Weight Estimate (1.5m scale)

Strut body weight scales linearly with length. Mockup strut body: 4.31 kg at 1,419mm avg = 3.04 g/mm.

| Component | Calculation | Total (kg) |
|-----------|-------------|----------:|
| Strut tubes | 146.4m total × 3.04 g/mm | 445 |
| Connector heads | 84 × 2.28 + 8 × 1.93 | 207 |
| Dyneema | 306.2m × 0.11 kg/m | 34 |
| Quick links | 360 × 0.29 kg | 104 |
| **Total** | | **~790 kg** |

### Comparison

| | Mockup | OpenClaw | Ratio |
|-|-------:|---------:|------:|
| Scale | 0.59m | 1.5m | 2.5× |
| Struts | 9 | 46 | 5.1× |
| Cables | 36 | 180 | 5.0× |
| Weight | 105 kg | ~790 kg | 7.5× |
| Material cost | €3,713 | €19,927 | 5.4× |
| Labor cost | €1,157 | €5,798 | 5.0× |
| **Total cost** | **€4,870** | **€25,725** | **5.3×** |

### Caveats

- **Tube sizing**: The Mockup uses 60x4mm aluminum outer tubes for struts up to ~1.55m. OpenClaw struts reach 4.86m — larger tube diameters may be needed for stiffness, which would increase cost and weight.
- **Shipping**: Estimated higher than Mockup due to larger/heavier parts. Actual cost depends on supplier and location.
- **Scale flexibility**: All tube and cable costs scale linearly. At a smaller scale (e.g., 0.59m like Mockup), the OpenClaw would cost roughly €13,000-€15,000 and weigh roughly 400-500 kg, with shorter struts that fit the existing 60x4mm tube specification.

---

*Source data: "Overzicht kosten Tensegrity Mockup" spreadsheets (Dutch), translated and reorganized for estimation use. OpenClaw counts obtained from simulation build.*
