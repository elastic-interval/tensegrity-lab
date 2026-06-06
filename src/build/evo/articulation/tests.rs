#[cfg(test)]
mod tests {
    use crate::build::dsl::brick_dsl::BrickName;
    use crate::build::evo::articulation::fitness::{breakdown, score};
    use crate::build::evo::articulation::genome::ArticulationGenome;
    use crate::build::evo::articulation::structure::BrickStructure;
    use crate::build::evo::articulation::trial::{evaluate, ArticulationConfig};
    use crate::build::evo::simple_population::SimplePopulation;
    use crate::build::evo::traits::{Genome, PopulationStrategy};
    use crate::fabric::interval::Role;
    use crate::fabric::physics::presets::BAKING;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn seed_expresses_with_faces_and_actuator() {
        let structure = BrickStructure::from_single_twist_left();
        let expressed = structure.express("test".to_string());
        assert_eq!(expressed.structural_joints.len(), 6, "structural joints");
        assert!(expressed.fabric.joints.len() >= 8, "incl. face middles");
        assert_eq!(expressed.actuators.len(), 1, "one actuator");
        assert_eq!(structure.faces.len(), 2, "single twist has two faces");
        let pushes = expressed
            .fabric
            .intervals
            .values()
            .filter(|i| i.has_role(Role::Pushing))
            .count();
        assert_eq!(pushes, 3);
    }

    #[test]
    fn mutations_yield_variants() {
        let genome = ArticulationGenome::seed();
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let variants = genome.adjacent_possible(&mut rng);
        assert!(!variants.is_empty(), "expected at least one variant");
        for v in &variants {
            assert_ne!(v.id().0, genome.id().0, "variant should have fresh id");
        }
    }

    #[test]
    fn trial_completes_and_is_finite() {
        let structure = BrickStructure::from_single_twist_left();
        let config = ArticulationConfig::new(BAKING);
        let outcome = evaluate(&structure, &config);
        assert!(outcome.finite, "seed should not blow up");
    }

    /// Diagnostic — prints seed numbers so the thresholds in trial.rs and
    /// fitness.rs can be tuned. Run with:
    /// `cargo test --release --lib articulation::tests::diagnose_seeds -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn diagnose_seeds() {
        let config = ArticulationConfig::new(BAKING);
        for name in [
            BrickName::OmniSymmetrical,
            BrickName::OmniTetrahedral,
            BrickName::SingleTwistLeft,
            BrickName::TorqueSymmetrical,
        ] {
            let structure = BrickStructure::from_baked(name);
            let b = breakdown(&evaluate(&structure, &config));
            let s = structure.express("x".to_string());
            println!(
                "{name:?}: joints={} faces={} -> gate={} gain={:.2} rev={:.2} score={:.3}",
                s.structural_joints.len(),
                structure.faces.len(),
                b.gate,
                b.gain,
                b.reversibility,
                b.score,
            );
        }
    }

    /// Can topology actually grow and stay valid? Apply random mutation
    /// chains to the omni seed and report how many still pass the gate and
    /// their face counts / scores.
    /// `cargo test --release --lib diagnose_growth -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn diagnose_growth() {
        let config = ArticulationConfig::new(BAKING);
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        let seed = ArticulationGenome::seed();
        let (mut gated, mut total) = (0, 0);
        let mut best = (0.0f32, 8usize);
        for _ in 0..40 {
            // chain several mutations to actually change topology
            let mut g = seed.clone();
            for _ in 0..6 {
                if let Some(v) = g.adjacent_possible(&mut rng).into_iter().next() {
                    g = v;
                }
            }
            let b = breakdown(&evaluate(g.structure(), &config));
            total += 1;
            if b.gate {
                gated += 1;
                if b.score > best.0 {
                    best = (b.score, g.structure().faces.len());
                }
            }
        }
        println!(
            "grown chains: {gated}/{total} passed gate; best score={:.3} at {} faces",
            best.0, best.1
        );
    }

    /// Trace KE decay over a long dormant settle to find the real
    /// "settled" energy scale and whether a standalone brick converges.
    #[test]
    #[ignore]
    fn diagnose_settle() {
        use crate::fabric::physics::presets::{BAKING, PRETENSING};
        use crate::Age;
        for (label, physics) in [("PRETENSING", PRETENSING), ("BAKING", BAKING)] {
            for name in [BrickName::SingleTwistLeft, BrickName::OmniSymmetrical] {
                let structure = BrickStructure::from_baked(name);
                let mut fabric = structure.express("settle".to_string()).fabric;
                let per_half_sec = (0.5 / Age::iteration_duration()) as usize;
                println!("=== {label} {name:?} settle KE ===");
                for half in 0..8 {
                    for _ in 0..per_half_sec {
                        fabric.iterate(&physics);
                    }
                    println!(
                        "  t={:.1}s  ke={:.8}",
                        (half + 1) as f32 * 0.5,
                        fabric.kinetic_energy()
                    );
                }
            }
        }
    }

    /// Watch the climb: prints best score per generation and the winning
    /// genome, so we can see whether the GA actually improves articulation
    /// and what it evolves toward.
    /// `cargo test --release --lib diagnose_evolution -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn diagnose_evolution() {
        let mut pop = SimplePopulation::new(24, 6, 7);
        pop.initialize(vec![ArticulationGenome::seed()]);
        let config = ArticulationConfig::new(BAKING);
        let mut overall_best: Option<(ArticulationGenome, f32)> = None;
        for gen in 0..20 {
            let mut best_gen = 0.0f32;
            while let Some(genome) = pop.next_for_trial() {
                let s = score(&evaluate(genome.structure(), &config));
                best_gen = best_gen.max(s);
                if overall_best.as_ref().map(|(_, b)| s > *b).unwrap_or(true) {
                    overall_best = Some((genome.clone(), s));
                }
                pop.record_result(genome, s);
            }
            println!("gen {gen:2}: best_this_gen={best_gen:.4}");
            pop.advance_generation();
        }
        if let Some((genome, s)) = overall_best {
            let outcome = evaluate(genome.structure(), &config);
            let radials: Vec<f32> = genome
                .structure()
                .faces
                .iter()
                .map(|f| (f.radial_strain * 1000.0).round() / 1000.0)
                .collect();
            let pushes: Vec<f32> = genome
                .structure()
                .pushes
                .iter()
                .map(|p| (p.ideal * 100.0).round() / 100.0)
                .collect();
            let raw_gain = outcome.deformation / (outcome.effort + 0.02);
            println!(
                "\nWINNER score={s:.4}  raw_gain={raw_gain:.3}\n  {}\n  radial_strains={radials:?}\n  push_ideals={pushes:?}\n  {:#?}\n  {:#?}",
                genome.describe(),
                outcome,
                breakdown(&outcome)
            );
        }
    }

    /// Headless: best fitness over several generations should beat the
    /// first generation's median.
    #[test]
    #[ignore]
    fn evolution_improves_fitness() {
        let mut pop = SimplePopulation::new(16, 4, 7);
        pop.initialize(vec![ArticulationGenome::seed()]);
        let config = ArticulationConfig::new(BAKING);

        let mut gen0: Vec<f32> = Vec::new();
        let mut best = 0.0f32;
        for gen in 0..6 {
            let mut scores = Vec::new();
            while let Some(genome) = pop.next_for_trial() {
                let s = score(&evaluate(genome.structure(), &config));
                scores.push(s);
                pop.record_result(genome, s);
            }
            if gen == 0 {
                gen0 = scores.clone();
            }
            best = best.max(scores.iter().cloned().fold(0.0, f32::max));
            pop.advance_generation();
        }

        gen0.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median0 = gen0[gen0.len() / 2];
        assert!(
            best >= median0,
            "best {best} should beat gen-0 median {median0}"
        );
    }
}
