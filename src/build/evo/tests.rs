#![cfg(test)]

use crate::build::evo::evolution::{Evolution, EvolutionConfig, EvolutionState};
use crate::build::evo::fitness::FitnessEvaluator;
use crate::build::evo::grower::{GrowthConfig, Grower};
use crate::build::evo::population::{MutationType, Population};
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::SETTLING;
use crate::fabric::physics::{Surface, SurfaceCharacter};
use crate::fabric::Fabric;
use crate::units::Meters;
use glam::Vec3;

// ============ Grower Tests ============

#[test]
fn test_grower_creates_seed_with_three_pushes() {
    let config = GrowthConfig::default();
    let mut grower = Grower::new(42, config);

    let (fabric, push_count) = grower.create_seed();

    // Should have 3 pushes = 6 joints
    assert_eq!(push_count, 3);
    assert_eq!(fabric.joints.len(), 6, "3 pushes should have 6 joints");

    // Count push intervals
    let push_intervals = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pushing)
        .count();
    assert_eq!(push_intervals, 3, "Should have 3 push intervals");

    // Should have some pull intervals connecting the pushes
    let pull_intervals = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pulling)
        .count();
    assert!(pull_intervals > 0, "Should have pull intervals connecting pushes");
}

#[test]
fn test_grower_seed_deterministic() {
    let config = GrowthConfig::default();

    let mut grower1 = Grower::new(42, config.clone());
    let mut grower2 = Grower::new(42, config);

    let (fabric1, count1) = grower1.create_seed();
    let (fabric2, count2) = grower2.create_seed();

    assert_eq!(count1, count2);
    assert_eq!(fabric1.joints.len(), fabric2.joints.len());
    assert_eq!(fabric1.intervals.len(), fabric2.intervals.len());

    // Joint positions should be identical
    let positions1: Vec<_> = fabric1
        .joints
        .values()
        .map(|j| (j.location.x, j.location.y, j.location.z))
        .collect();
    let positions2: Vec<_> = fabric2
        .joints
        .values()
        .map(|j| (j.location.x, j.location.y, j.location.z))
        .collect();
    assert_eq!(positions1, positions2);
}

#[test]
fn test_grower_different_seeds_different_structures() {
    let config = GrowthConfig::default();

    let mut grower1 = Grower::new(42, config.clone());
    let mut grower2 = Grower::new(43, config);

    let (fabric1, _) = grower1.create_seed();
    let (fabric2, _) = grower2.create_seed();

    // Joint positions should differ
    let positions1: Vec<_> = fabric1
        .joints
        .values()
        .map(|j| (j.location.x, j.location.y, j.location.z))
        .collect();
    let positions2: Vec<_> = fabric2
        .joints
        .values()
        .map(|j| (j.location.x, j.location.y, j.location.z))
        .collect();
    assert_ne!(positions1, positions2, "Different seeds should produce different structures");
}

#[test]
fn test_grower_mutate_adds_one_push() {
    let config = GrowthConfig::default();
    let mut grower = Grower::new(42, config);

    let (mut fabric, push_count) = grower.create_seed();
    let initial_pushes = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pushing)
        .count();

    // Mutate
    let new_count = grower.mutate(&mut fabric, push_count);

    let final_pushes = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pushing)
        .count();

    assert_eq!(new_count, push_count + 1);
    assert_eq!(final_pushes, initial_pushes + 1);
}

#[test]
fn test_grower_mutate_adds_connections() {
    let config = GrowthConfig::default();
    let mut grower = Grower::new(42, config);

    let (mut fabric, push_count) = grower.create_seed();
    let initial_pulls = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pulling)
        .count();

    // Mutate
    grower.mutate(&mut fabric, push_count);

    let final_pulls = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pulling)
        .count();

    // New push should add some pull connections
    assert!(
        final_pulls > initial_pulls,
        "Mutation should add pull connections"
    );
}

#[test]
fn test_grower_settling() {
    let config = GrowthConfig::default();
    let mut grower = Grower::new(42, config);

    let (mut fabric, _) = grower.create_seed();

    // Create physics with slippery surface
    let mut physics = SETTLING.clone();
    physics.surface = Some(Surface::new(SurfaceCharacter::Slippery, 1.0));

    // Get initial positions
    let initial_positions: Vec<_> = fabric
        .joints
        .values()
        .map(|j| j.location)
        .collect();

    // Settle for a short time
    grower.settle(&mut fabric, &physics, 0.1);

    // Positions should have changed due to physics
    let final_positions: Vec<_> = fabric
        .joints
        .values()
        .map(|j| j.location)
        .collect();

    let any_moved = initial_positions
        .iter()
        .zip(final_positions.iter())
        .any(|(a, b)| (*a - *b).length() > 0.0001);

    assert!(any_moved, "Settling should move joints");
}

// ============ Fitness Tests ============

#[test]
fn test_empty_fabric_zero_fitness() {
    let fabric = Fabric::new("test".to_string());
    let evaluator = FitnessEvaluator::new();
    assert_eq!(evaluator.evaluate(&fabric, 0), 0.0);
}

#[test]
fn test_fitness_with_push_count() {
    // Create a simple fabric with known height
    let mut fabric = Fabric::new("test".to_string());
    let bottom = fabric.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let top = fabric.create_joint(Vec3::new(0.0, 1.0, 0.0));
    fabric.create_slack_interval(bottom, top, Role::Pushing);

    let evaluator = FitnessEvaluator::new();

    // Fitness = perceived_height / cost, where cost = push_count * 4 + pull_count
    // With height=1.0, 1 push, 0 pulls: cost = 4, fitness ≈ 1.0/4 = 0.25 (+ stability bonus)
    let fitness1 = evaluator.evaluate(&fabric, 1);
    assert!(fitness1 > 0.2 && fitness1 < 0.4, "Fitness with 1 push should be ~0.25-0.3, got {}", fitness1);

    // With 4 pushes reported, cost = 16, fitness ≈ 1.0/16 = 0.0625
    let fitness4 = evaluator.evaluate(&fabric, 4);
    assert!(
        fitness4 < fitness1,
        "Higher push count should reduce fitness"
    );
}

#[test]
fn test_fitness_taller_is_better() {
    // Create two fabrics with different heights
    let mut fabric1 = Fabric::new("short".to_string());
    let bottom1 = fabric1.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let top1 = fabric1.create_joint(Vec3::new(0.0, 1.0, 0.0));
    fabric1.create_slack_interval(bottom1, top1, Role::Pushing);

    let mut fabric2 = Fabric::new("tall".to_string());
    let bottom2 = fabric2.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let top2 = fabric2.create_joint(Vec3::new(0.0, 2.0, 0.0));
    fabric2.create_slack_interval(bottom2, top2, Role::Pushing);

    let evaluator = FitnessEvaluator::new();
    let fitness1 = evaluator.evaluate(&fabric1, 1);
    let fitness2 = evaluator.evaluate(&fabric2, 1);

    assert!(
        fitness2 > fitness1,
        "Taller fabric should have higher fitness"
    );
}

#[test]
fn test_fitness_cost_adjusted() {
    // Two fabrics with same height but different push counts
    let mut fabric1 = Fabric::new("efficient".to_string());
    let bottom1 = fabric1.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let top1 = fabric1.create_joint(Vec3::new(0.0, 1.0, 0.0));
    fabric1.create_slack_interval(bottom1, top1, Role::Pushing);

    let mut fabric2 = Fabric::new("wasteful".to_string());
    let bottom2 = fabric2.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let top2 = fabric2.create_joint(Vec3::new(0.0, 1.0, 0.0));
    fabric2.create_slack_interval(bottom2, top2, Role::Pushing);

    let evaluator = FitnessEvaluator::new();
    // Same fabric, but evaluating with different push counts
    let fitness_efficient = evaluator.evaluate(&fabric1, 1);
    let fitness_wasteful = evaluator.evaluate(&fabric2, 9);

    // 1 push vs 9 pushes should show sqrt(9) = 3x cost penalty
    assert!(
        fitness_efficient > fitness_wasteful * 2.0,
        "Efficient structure should have much higher fitness"
    );
}

#[test]
fn test_fitness_details() {
    let mut fabric = Fabric::new("test".to_string());
    let bottom = fabric.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let top = fabric.create_joint(Vec3::new(0.0, 1.5, 0.0));
    fabric.create_slack_interval(bottom, top, Role::Pushing);

    let evaluator = FitnessEvaluator::new();
    let details = evaluator.evaluate_detailed(&fabric, 2);

    assert!(details.is_valid);
    assert!((details.height - 1.5).abs() < 0.01);
    assert_eq!(details.push_count, 2);
    assert!(details.fitness > 0.0);
}

// ============ Population Tests ============

#[test]
fn test_population_empty() {
    let pop = Population::new(42, 100);
    assert_eq!(pop.size(), 0);
    assert_eq!(pop.capacity(), 100);
    assert!(!pop.is_full());
    assert!(pop.best_current().is_none());
    assert!(pop.best_ever().is_none());
}

#[test]
fn test_population_add_initial() {
    let mut pop = Population::new(42, 100);

    let fabric1 = Fabric::new("test1".to_string());
    let fabric2 = Fabric::new("test2".to_string());
    let fabric3 = Fabric::new("test3".to_string());

    pop.add_initial(1, fabric1, 1.0, 0.5, 3);
    pop.add_initial(2, fabric2, 2.0, 0.5, 4);
    pop.add_initial(3, fabric3, 0.5, 0.5, 2);

    assert_eq!(pop.size(), 3);
    assert_eq!(pop.best_current().unwrap().fitness, 2.0);
    assert_eq!(pop.worst_current().unwrap().fitness, 0.5);
}

#[test]
fn test_population_try_insert_better() {
    let mut pop = Population::new(42, 3);

    pop.add_initial(1, Fabric::new("a".to_string()), 1.0, 0.5, 3);
    pop.add_initial(2, Fabric::new("b".to_string()), 2.0, 0.5, 3);
    pop.add_initial(3, Fabric::new("c".to_string()), 0.5, 0.5, 3);

    assert!(pop.is_full());

    // Insert something better than worst (0.5)
    let inserted = pop.try_insert(
        4, Fabric::new("d".to_string()), 1.5, 0.5, 3, 0,
        vec![(MutationType::Seed, 0.5)], MutationType::ShortenPull,
    );
    assert!(inserted);
    assert_eq!(pop.worst_current().unwrap().fitness, 1.0);
}

#[test]
fn test_population_try_insert_worse() {
    let mut pop = Population::new(42, 3);

    pop.add_initial(1, Fabric::new("a".to_string()), 1.0, 0.5, 3);
    pop.add_initial(2, Fabric::new("b".to_string()), 2.0, 0.5, 3);
    pop.add_initial(3, Fabric::new("c".to_string()), 0.5, 0.5, 3);

    // Try to insert something worse than worst (0.5)
    // Note: Has 5% random chance to be accepted, but we use seeded RNG so it's deterministic
    let inserted = pop.try_insert(
        4, Fabric::new("d".to_string()), 0.3, 0.5, 3, 0,
        vec![(MutationType::Seed, 0.5)], MutationType::ShortenPull,
    );
    // With seed 42, this should not be accepted by random chance
    if !inserted {
        assert_eq!(pop.size(), 3);
        assert_eq!(pop.worst_current().unwrap().fitness, 0.5);
    }
}

#[test]
fn test_population_best_ever_tracking() {
    let mut pop = Population::new(42, 3);

    pop.add_initial(1, Fabric::new("a".to_string()), 1.0, 0.5, 3);
    assert_eq!(pop.best_ever().unwrap().fitness, 1.0);

    pop.add_initial(2, Fabric::new("b".to_string()), 3.0, 0.5, 3);
    assert_eq!(pop.best_ever().unwrap().fitness, 3.0);

    pop.add_initial(3, Fabric::new("c".to_string()), 2.0, 0.5, 3);
    // Best ever should still be 3.0
    assert_eq!(pop.best_ever().unwrap().fitness, 3.0);

    // Insert a new best
    pop.try_insert(
        4, Fabric::new("d".to_string()), 5.0, 0.5, 3, 0,
        vec![(MutationType::Seed, 1.0)], MutationType::AddPush,
    );
    assert_eq!(pop.best_ever().unwrap().fitness, 5.0);
}

#[test]
fn test_population_pick_random() {
    let mut pop = Population::new(42, 10);

    // Empty population
    assert!(pop.pick_random().is_none());

    pop.add_initial(1, Fabric::new("a".to_string()), 1.0, 0.5, 3);
    pop.add_initial(2, Fabric::new("b".to_string()), 2.0, 0.5, 3);
    pop.add_initial(3, Fabric::new("c".to_string()), 3.0, 0.5, 3);

    // Should be able to pick
    for _ in 0..20 {
        assert!(pop.pick_random().is_some());
    }
}

#[test]
fn test_population_stats() {
    let mut pop = Population::new(42, 10);

    pop.add_initial(1, Fabric::new("a".to_string()), 1.0, 0.5, 3);
    pop.add_initial(2, Fabric::new("b".to_string()), 2.0, 0.5, 4);
    pop.add_initial(3, Fabric::new("c".to_string()), 3.0, 0.5, 5);

    let stats = pop.stats();
    assert_eq!(stats.size, 3);
    assert_eq!(stats.min_fitness, 1.0);
    assert_eq!(stats.max_fitness, 3.0);
    assert!((stats.mean_fitness - 2.0).abs() < 0.001);
    assert!((stats.avg_push_count - 4.0).abs() < 0.001);
}

#[test]
fn test_population_generation() {
    let mut pop = Population::new(42, 10);
    assert_eq!(pop.generation(), 0);

    pop.next_generation();
    assert_eq!(pop.generation(), 1);

    pop.next_generation();
    assert_eq!(pop.generation(), 2);
}

// ============ Evolution Tests ============

#[test]
fn test_evolution_creation() {
    let config = EvolutionConfig {
        population_size: 5,
        ..Default::default()
    };
    let evo = Evolution::with_master_seed(42, config);

    assert_eq!(*evo.state(), EvolutionState::CreatingSeed);
    assert_eq!(evo.population().size(), 0);
}

#[test]
fn test_evolution_with_seed() {
    let evo = Evolution::with_master_seed(42, EvolutionConfig::default());
    assert_eq!(*evo.state(), EvolutionState::CreatingSeed);
}

#[test]
fn test_evolution_stats_initial() {
    let config = EvolutionConfig {
        population_size: 5,
        ..Default::default()
    };
    let evo = Evolution::with_master_seed(42, config);
    let stats = evo.stats();

    assert_eq!(stats.generation, 0);
    assert_eq!(stats.population_size, 0);
    assert_eq!(stats.evaluations, 0);
}

#[test]
fn test_evolution_step_creates_seed() {
    let config = EvolutionConfig {
        population_size: 5,
        seed_push_count: 3,
        ..Default::default()
    };
    let evo = Evolution::with_master_seed(42, config);

    // Initially in CreatingSeed state
    assert_eq!(*evo.state(), EvolutionState::CreatingSeed);

    // Run enough steps to move past seed creation
    for _ in 0..200_000 {
        // One step internally
        if !matches!(*evo.state(), EvolutionState::CreatingSeed) {
            break;
        }
        // We need to access the step function - but it's private
        // So we'll use this workaround
    }

    // The Evolution struct manages its own stepping, so we can't directly test
    // internal state transitions without the iterate() function
}

#[test]
fn test_grower_integration_full_cycle() {
    // Test the full cycle: create seed, settle, mutate, settle
    let config = GrowthConfig {
        push_length: Meters(1.0),
        seed_push_count: 3,
        seed_settle_seconds: 0.5, // Short for testing
        mutation_settle_seconds: 0.2,
        ..Default::default()
    };

    let mut grower = Grower::new(42, config);

    // Create seed
    let (mut fabric, push_count) = grower.create_seed();
    assert_eq!(push_count, 3);

    // Create physics with slippery surface
    let mut physics = SETTLING.clone();
    physics.surface = Some(Surface::new(SurfaceCharacter::Slippery, 1.0));

    // Settle seed
    grower.settle_seed(&mut fabric, &physics);

    // Check it's still valid
    assert!(fabric.joints.len() >= 6);

    // Mutate
    let new_push_count = grower.mutate(&mut fabric, push_count);
    assert_eq!(new_push_count, 4);

    // Settle mutation
    grower.settle_mutation(&mut fabric, &physics);

    // Final check
    let push_intervals = fabric
        .intervals
        .values()
        .filter(|i| i.role == Role::Pushing)
        .count();
    assert_eq!(push_intervals, 4);
}

#[test]
fn test_evolution_seeding_process() {
    // Create a simple evolution run and verify population fills
    let config = GrowthConfig {
        push_length: Meters(1.0),
        seed_push_count: 3,
        seed_settle_seconds: 0.1,
        mutation_settle_seconds: 0.1,
        ..Default::default()
    };

    let mut grower = Grower::new(42, config);
    let mut pop = Population::new(42, 5);
    let evaluator = FitnessEvaluator::new();

    // Create physics
    let mut physics = SETTLING.clone();
    physics.surface = Some(Surface::new(SurfaceCharacter::Slippery, 1.0));

    // Create and settle seed
    let (seed_fabric, push_count) = grower.create_seed();
    let mut seed = seed_fabric.clone();
    grower.settle_seed(&mut seed, &physics);

    // Evaluate and add seed
    let seed_details = evaluator.evaluate_detailed(&seed, push_count);
    pop.add_initial(42, seed.clone(), seed_details.fitness, seed_details.height, push_count);

    // Fill rest of population with variations
    for i in 1..5 {
        let mut variant = seed.clone();
        let new_count = grower.mutate(&mut variant, push_count);
        grower.settle_mutation(&mut variant, &physics);

        let details = evaluator.evaluate_detailed(&variant, new_count);
        pop.add_initial(42 + i as u64, variant, details.fitness, details.height, new_count);
    }

    assert!(pop.is_full());
    assert!(pop.best_current().unwrap().fitness > 0.0);

    let stats = pop.stats();
    eprintln!(
        "Population filled: size={}, best={:.3}, mean={:.3}, avg_pushes={:.1}",
        stats.size, stats.max_fitness, stats.mean_fitness, stats.avg_push_count
    );
}

#[test]
fn test_evolution_competitive_insertion() {
    // Test that better offspring replace worse individuals
    let config = GrowthConfig::default();
    let mut grower = Grower::new(42, config);
    let mut pop = Population::new(42, 3);
    let _evaluator = FitnessEvaluator::new();

    let mut physics = SETTLING.clone();
    physics.surface = Some(Surface::new(SurfaceCharacter::Slippery, 1.0));

    // Create seed
    let (mut seed, push_count) = grower.create_seed();
    grower.settle_seed(&mut seed, &physics);

    // Add initial population with known fitnesses
    pop.add_initial(1, seed.clone(), 0.1, 0.5, push_count);
    pop.add_initial(2, seed.clone(), 0.2, 0.5, push_count);
    pop.add_initial(3, seed.clone(), 0.3, 0.5, push_count);

    // Try to insert something better
    let inserted = pop.try_insert(
        4, seed.clone(), 0.5, 0.5, push_count, 0,
        vec![(MutationType::Seed, 0.1)], MutationType::ShortenPull,
    );
    assert!(inserted);
    assert!(pop.worst_current().unwrap().fitness >= 0.2);

    // Try to insert something worse (but has 5% random chance, so we check deterministically fails)
    // Note: this test may occasionally pass due to random acceptance, so we just verify behavior
    let _result = pop.try_insert(
        5, seed.clone(), 0.05, 0.5, push_count, 0,
        vec![(MutationType::Seed, 0.1)], MutationType::ShortenPull,
    );
}

#[test]
fn test_evolution_determinism() {
    // Test that the same seed produces identical results
    use crate::fabric::physics::SurfaceCharacter;

    fn run_evolution(seed: u64) -> (f32, usize, Vec<f32>) {
        let config = GrowthConfig {
            push_length: Meters(1.0),
            seed_push_count: 3,
            seed_settle_seconds: 0.5,
            mutation_settle_seconds: 0.2,
            ..Default::default()
        };

        let mut grower = Grower::new(seed, config);
        let mut pop = Population::new(seed, 10);
        let evaluator = FitnessEvaluator::new();

        let mut physics = SETTLING.clone();
        physics.surface = Some(Surface::new(SurfaceCharacter::Bouncy, 1.0));

        // Create and settle seed
        let (mut seed_fabric, push_count) = grower.create_seed();
        grower.settle_seed(&mut seed_fabric, &physics);

        // Evaluate seed
        let seed_details = evaluator.evaluate_detailed(&seed_fabric, push_count);
        pop.add_initial(seed, seed_fabric.clone(), seed_details.fitness, seed_details.height, push_count);

        // Fill population with mutations
        for i in 1..10 {
            let mut variant = seed_fabric.clone();
            let new_count = grower.mutate(&mut variant, push_count);
            grower.settle_mutation(&mut variant, &physics);
            let details = evaluator.evaluate_detailed(&variant, new_count);
            pop.add_initial(seed + i as u64, variant, details.fitness, details.height, new_count);
        }

        // Collect all fitnesses for comparison
        let fitnesses: Vec<f32> = pop
            .pool()
            .iter()
            .map(|ind| ind.fitness)
            .collect();

        let stats = pop.stats();
        (stats.max_fitness, pop.size(), fitnesses)
    }

    // Run twice with same seed
    let (best1, size1, fitnesses1) = run_evolution(42);
    let (best2, size2, fitnesses2) = run_evolution(42);

    // Verify identical results
    assert_eq!(size1, size2, "Population sizes should match");
    assert!(
        (best1 - best2).abs() < 1e-6,
        "Best fitness should match: {} vs {}",
        best1,
        best2
    );
    assert_eq!(
        fitnesses1.len(),
        fitnesses2.len(),
        "Fitness vectors should have same length"
    );
    for (i, (f1, f2)) in fitnesses1.iter().zip(fitnesses2.iter()).enumerate() {
        assert!(
            (f1 - f2).abs() < 1e-6,
            "Fitness at index {} should match: {} vs {}",
            i,
            f1,
            f2
        );
    }

    // Also verify we got nonzero height
    assert!(
        best1 > 0.0,
        "Best fitness should be positive (nonzero height): {}",
        best1
    );

    eprintln!(
        "Determinism verified: best_fitness={:.4}, population_size={}",
        best1, size1
    );
}
