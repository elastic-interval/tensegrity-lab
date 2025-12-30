#![cfg(test)]

use crate::build::evo::cell::Cell;
use crate::build::evo::decision_maker::DecisionMaker;
use crate::build::evo::evolution::{Evolution, EvolutionConfig, EvolutionState};
use crate::build::evo::fitness::FitnessEvaluator;
use crate::build::evo::genome::Genome;
use crate::build::evo::grower::{GrowthConfig, GrowthResult, Grower};
use crate::build::evo::population::Population;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd};
use glam::Vec3;

// ============ Genome Tests ============

#[test]
fn test_empty_genome_no_skips() {
    let genome = Genome::new();
    for i in 0..100 {
        assert!(!genome.should_skip(i), "Position {} should not be skipped", i);
    }
}

#[test]
fn test_single_skip() {
    let genome = Genome::from_offsets(vec![5]);
    assert!(!genome.should_skip(0));
    assert!(!genome.should_skip(4));
    assert!(genome.should_skip(5));
    assert!(!genome.should_skip(6));
}

#[test]
fn test_multiple_skips() {
    // Skip at 5, 17 (5+12), 20 (17+3)
    let genome = Genome::from_offsets(vec![5, 12, 3]);
    assert!(genome.should_skip(5));
    assert!(genome.should_skip(17));
    assert!(genome.should_skip(20));
    assert!(!genome.should_skip(6));
    assert!(!genome.should_skip(16));
    assert!(!genome.should_skip(18));
}

#[test]
fn test_with_skip_at_beginning() {
    let genome = Genome::from_offsets(vec![10]);
    let mutated = genome.with_skip_at(3);
    assert_eq!(mutated.skip_positions(), vec![3, 10]);
}

#[test]
fn test_with_skip_at_middle() {
    let genome = Genome::from_offsets(vec![5, 10]); // skips at 5, 15
    let mutated = genome.with_skip_at(8);
    assert_eq!(mutated.skip_positions(), vec![5, 8, 15]);
}

#[test]
fn test_with_skip_at_end() {
    let genome = Genome::from_offsets(vec![5]); // skip at 5
    let mutated = genome.with_skip_at(20);
    assert_eq!(mutated.skip_positions(), vec![5, 20]);
}

#[test]
fn test_skip_positions() {
    let genome = Genome::from_offsets(vec![5, 12, 3]);
    assert_eq!(genome.skip_positions(), vec![5, 17, 20]);
}

#[test]
fn test_rle_long_gap() {
    // Test skip at position 522 (256 + 256 + 10)
    let genome = Genome::new();
    let mutated = genome.with_skip_at(522);
    assert!(mutated.should_skip(522));
    assert!(!mutated.should_skip(521));
    assert!(!mutated.should_skip(523));
    // Should use RLE encoding: [0, 0, 10]
    assert_eq!(mutated.skip_positions(), vec![522]);
}

#[test]
fn test_rle_encoding() {
    // Direct RLE encoding test: [0, 0, 10] = skip at 256+256+10 = 522
    let genome = Genome::from_offsets(vec![0, 0, 10]);
    assert_eq!(genome.skip_positions(), vec![522]);
    assert!(genome.should_skip(522));
}

// ============ DecisionMaker Tests ============

#[test]
fn test_determinism_same_seed_same_genome() {
    let genome = Genome::new();
    let seed = 42u64;

    let mut dm1 = DecisionMaker::new(seed, genome.clone());
    let mut dm2 = DecisionMaker::new(seed, genome.clone());

    for _ in 0..100 {
        assert_eq!(dm1.decide(), dm2.decide());
    }
}

#[test]
fn test_different_seeds_different_results() {
    let genome = Genome::new();

    let mut dm1 = DecisionMaker::new(42, genome.clone());
    let mut dm2 = DecisionMaker::new(43, genome.clone());

    // Collect results
    let results1: Vec<bool> = (0..20).map(|_| dm1.decide()).collect();
    let results2: Vec<bool> = (0..20).map(|_| dm2.decide()).collect();

    // Should differ somewhere
    assert_ne!(results1, results2);
}

#[test]
fn test_skip_changes_sequence() {
    let genome1 = Genome::new();
    let genome2 = genome1.with_skip_at(5);
    let seed = 42u64;

    let mut dm1 = DecisionMaker::new(seed, genome1);
    let mut dm2 = DecisionMaker::new(seed, genome2);

    // First 5 decisions should match
    for i in 0..5 {
        assert_eq!(
            dm1.decide(),
            dm2.decide(),
            "Decision {} should match before skip",
            i
        );
    }

    // After skip point, at least one should differ in next 20
    let mut any_different = false;
    for _ in 0..20 {
        if dm1.decide() != dm2.decide() {
            any_different = true;
            break;
        }
    }
    assert!(any_different, "Sequences should diverge after skip");
}

#[test]
fn test_choose_range() {
    let mut dm = DecisionMaker::new(42, Genome::new());

    for _ in 0..100 {
        let choice = dm.choose(10);
        assert!(choice < 10, "Choice should be in range [0, 10)");
    }
}

#[test]
fn test_random_direction_normalized() {
    let mut dm = DecisionMaker::new(42, Genome::new());

    for _ in 0..100 {
        let dir = dm.random_direction();
        let len = dir.length();
        // Should be normalized (length ~1.0) or zero vector
        assert!(
            (len - 1.0).abs() < 0.001 || len < 0.001,
            "Direction should be normalized, got length {}",
            len
        );
    }
}

#[test]
fn test_virtual_position_increments() {
    let mut dm = DecisionMaker::new(42, Genome::new());
    assert_eq!(dm.virtual_position(), 0);

    dm.decide();
    assert_eq!(dm.virtual_position(), 1);

    dm.decide();
    assert_eq!(dm.virtual_position(), 2);
}

#[test]
fn test_virtual_position_with_skips() {
    // Genome with skip at position 2
    let genome = Genome::from_offsets(vec![2]);
    let mut dm = DecisionMaker::new(42, genome);

    dm.decide(); // Uses position 0
    assert_eq!(dm.virtual_position(), 1);

    dm.decide(); // Uses position 1
    assert_eq!(dm.virtual_position(), 2);

    dm.decide(); // Position 2 is skipped, uses position 3
    assert_eq!(dm.virtual_position(), 4);
}

// ============ Cell Tests ============

#[test]
fn test_cell_creation() {
    let mut fabric = Fabric::new("test".to_string());
    let cell = Cell::new(&mut fabric, Vec3::ZERO, Vec3::Y, 1.0);

    assert_eq!(fabric.joints.len(), 2);
    assert_eq!(fabric.intervals.len(), 1);
    assert_eq!(cell.total_pulls(), 0);
}

#[test]
fn test_pull_tracking() {
    let mut fabric = Fabric::new("test".to_string());
    let cell1 = Cell::new(&mut fabric, Vec3::ZERO, Vec3::Y, 1.0);
    let cell2 = Cell::new(&mut fabric, Vec3::new(1.0, 0.0, 0.0), Vec3::Y, 1.0);

    // Create a pull between cell1.alpha and cell2.alpha
    let pull = fabric.create_slack_interval(cell1.alpha_joint, cell2.alpha_joint, Role::Pulling);

    let mut cell1 = cell1;
    let mut cell2 = cell2;
    cell1.add_pull(IntervalEnd::Alpha, pull);
    cell2.add_pull(IntervalEnd::Alpha, pull);

    assert_eq!(cell1.pull_count(IntervalEnd::Alpha), 1);
    assert_eq!(cell1.pull_count(IntervalEnd::Omega), 0);
    assert!(cell1.needs_more_pulls(IntervalEnd::Alpha)); // < 3
    assert!(cell1.can_accept_pull(IntervalEnd::Alpha)); // < 6
}

#[test]
fn test_connection_limits() {
    let mut fabric = Fabric::new("test".to_string());
    let mut cell = Cell::new(&mut fabric, Vec3::ZERO, Vec3::Y, 1.0);

    // Add 6 dummy pulls
    for i in 0..6 {
        let dummy_joint = fabric.create_joint(Vec3::new(i as f32, 0.0, 0.0));
        let pull = fabric.create_slack_interval(cell.alpha_joint, dummy_joint, Role::Pulling);
        cell.add_pull(IntervalEnd::Alpha, pull);
    }

    assert_eq!(cell.pull_count(IntervalEnd::Alpha), 6);
    assert!(!cell.needs_more_pulls(IntervalEnd::Alpha)); // >= 3
    assert!(!cell.can_accept_pull(IntervalEnd::Alpha)); // >= 6
}

// ============ Grower Tests ============

#[test]
fn test_grower_creates_first_cell() {
    let config = GrowthConfig {
        max_steps: 10,
        ..Default::default()
    };
    let mut grower = Grower::new(42, Genome::new(), config);

    // First step should create first cell
    let result = grower.grow_step();
    assert!(matches!(result, GrowthResult::Continue));
    assert_eq!(grower.cells.len(), 1);
    assert_eq!(grower.fabric.joints.len(), 2);
    assert_eq!(grower.fabric.intervals.len(), 1);
}

#[test]
fn test_grower_deterministic() {
    let config = GrowthConfig {
        max_steps: 20,
        ..Default::default()
    };

    // Grow two structures with same seed and genome
    let mut grower1 = Grower::new(42, Genome::new(), config.clone());
    let mut grower2 = Grower::new(42, Genome::new(), config);

    grower1.grow_complete();
    grower2.grow_complete();

    // Should produce identical structures
    assert_eq!(grower1.cells.len(), grower2.cells.len());
    assert_eq!(grower1.fabric.joints.len(), grower2.fabric.joints.len());
    assert_eq!(grower1.fabric.intervals.len(), grower2.fabric.intervals.len());
}

#[test]
fn test_grower_skip_changes_structure() {
    let config = GrowthConfig {
        max_steps: 30,
        ..Default::default()
    };

    let genome1 = Genome::new();
    let genome2 = genome1.with_skip_at(5);

    let mut grower1 = Grower::new(42, genome1, config.clone());
    let mut grower2 = Grower::new(42, genome2, config);

    grower1.grow_complete();
    grower2.grow_complete();

    // Structures should differ due to skip
    // (At least intervals should be different since decisions diverge)
    let different = grower1.fabric.intervals.len() != grower2.fabric.intervals.len()
        || grower1.cells.len() != grower2.cells.len();
    assert!(
        different,
        "Skip should produce different structure: cells1={} cells2={} intervals1={} intervals2={}",
        grower1.cells.len(),
        grower2.cells.len(),
        grower1.fabric.intervals.len(),
        grower2.fabric.intervals.len()
    );
}

#[test]
fn test_grower_creates_connections() {
    let config = GrowthConfig {
        max_steps: 30,
        ..Default::default()
    };
    let mut grower = Grower::new(42, Genome::new(), config);

    grower.grow_complete();

    // Should have multiple cells and connections
    assert!(grower.cells.len() > 1, "Should have multiple cells");

    // Count total pull connections
    let total_pulls: usize = grower.cells.iter().map(|c| c.total_pulls()).sum();
    assert!(total_pulls > 0, "Should have pull connections");
}

#[test]
fn test_grower_completes_at_max_steps() {
    let config = GrowthConfig {
        max_steps: 5,
        ..Default::default()
    };
    let mut grower = Grower::new(42, Genome::new(), config);

    let result = grower.grow_complete();
    assert!(matches!(result, GrowthResult::Complete));
    assert_eq!(grower.growth_step, 5);
}

// ============ Fitness Tests ============

#[test]
fn test_empty_fabric_zero_fitness() {
    let fabric = Fabric::new("test".to_string());
    let evaluator = FitnessEvaluator::new();
    assert_eq!(evaluator.evaluate(&fabric), 0.0);
}

#[test]
fn test_single_cell_has_fitness() {
    let mut fabric = Fabric::new("test".to_string());
    let _cell = Cell::new(&mut fabric, Vec3::ZERO, Vec3::Y, 1.0);

    let evaluator = FitnessEvaluator::new();
    let fitness = evaluator.evaluate(&fabric);

    // Single vertical cell should have height ~1.0
    assert!(fitness > 0.5, "Fitness should be > 0.5, got {}", fitness);
}

#[test]
fn test_taller_structure_higher_fitness() {
    // Create two structures of different heights
    let mut fabric1 = Fabric::new("short".to_string());
    Cell::new(&mut fabric1, Vec3::ZERO, Vec3::Y, 1.0);

    let mut fabric2 = Fabric::new("tall".to_string());
    Cell::new(&mut fabric2, Vec3::ZERO, Vec3::Y, 2.0);

    let evaluator = FitnessEvaluator::new();
    let fitness1 = evaluator.evaluate(&fabric1);
    let fitness2 = evaluator.evaluate(&fabric2);

    assert!(
        fitness2 > fitness1,
        "Taller structure should have higher fitness: {} vs {}",
        fitness2,
        fitness1
    );
}

#[test]
fn test_grown_structure_fitness() {
    let config = GrowthConfig {
        max_steps: 30,
        ..Default::default()
    };
    let mut grower = Grower::new(42, Genome::new(), config);
    grower.grow_complete();

    let evaluator = FitnessEvaluator::new();
    let details = evaluator.evaluate_detailed(&grower.fabric);

    assert!(details.is_valid, "Grown structure should be valid");
    assert!(details.fitness > 0.0, "Grown structure should have positive fitness");
    assert!(details.height > 0.0, "Grown structure should have height");
}

#[test]
fn test_fitness_details() {
    let mut fabric = Fabric::new("test".to_string());
    Cell::new(&mut fabric, Vec3::ZERO, Vec3::Y, 1.0);

    let evaluator = FitnessEvaluator::new();
    let details = evaluator.evaluate_detailed(&fabric);

    assert!(details.is_valid);
    assert!(details.height > 0.9 && details.height < 1.1, "Height should be ~1.0");
    assert!(details.max_strain >= 0.0);
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

    pop.add_initial(Genome::new(), 1.0);
    pop.add_initial(Genome::new(), 2.0);
    pop.add_initial(Genome::new(), 0.5);

    assert_eq!(pop.size(), 3);
    assert_eq!(pop.best_current().unwrap().fitness, 2.0);
    assert_eq!(pop.worst_current().unwrap().fitness, 0.5);
}

#[test]
fn test_population_try_insert_better() {
    let mut pop = Population::new(42, 3);

    pop.add_initial(Genome::new(), 1.0);
    pop.add_initial(Genome::new(), 2.0);
    pop.add_initial(Genome::new(), 0.5);

    assert!(pop.is_full());

    // Insert something better than worst (0.5)
    let inserted = pop.try_insert(Genome::new(), 1.5);
    assert!(inserted);
    assert_eq!(pop.worst_current().unwrap().fitness, 1.0);
}

#[test]
fn test_population_try_insert_worse() {
    let mut pop = Population::new(42, 3);

    pop.add_initial(Genome::new(), 1.0);
    pop.add_initial(Genome::new(), 2.0);
    pop.add_initial(Genome::new(), 0.5);

    // Try to insert something worse than worst (0.5)
    let inserted = pop.try_insert(Genome::new(), 0.3);
    assert!(!inserted);
    assert_eq!(pop.size(), 3);
    assert_eq!(pop.worst_current().unwrap().fitness, 0.5);
}

#[test]
fn test_population_best_ever_tracking() {
    let mut pop = Population::new(42, 3);

    pop.add_initial(Genome::new(), 1.0);
    assert_eq!(pop.best_ever().unwrap().fitness, 1.0);

    pop.add_initial(Genome::new(), 3.0);
    assert_eq!(pop.best_ever().unwrap().fitness, 3.0);

    pop.add_initial(Genome::new(), 2.0);
    // Best ever should still be 3.0
    assert_eq!(pop.best_ever().unwrap().fitness, 3.0);

    // Insert a new best
    pop.try_insert(Genome::new(), 5.0);
    assert_eq!(pop.best_ever().unwrap().fitness, 5.0);
}

#[test]
fn test_population_pick_random() {
    let mut pop = Population::new(42, 10);

    // Empty population
    assert!(pop.pick_random().is_none());

    pop.add_initial(Genome::new(), 1.0);
    pop.add_initial(Genome::new(), 2.0);
    pop.add_initial(Genome::new(), 3.0);

    // Should be able to pick
    for _ in 0..20 {
        assert!(pop.pick_random().is_some());
    }
}

#[test]
fn test_population_pick_parent_genome() {
    let mut pop = Population::new(42, 10);

    // Create a genome with a skip
    let genome = Genome::from_offsets(vec![5]);
    pop.add_initial(genome, 1.0);

    let picked = pop.pick_parent_genome();
    assert!(picked.is_some());
    let picked_genome = picked.unwrap();
    assert_eq!(picked_genome.skip_positions(), vec![5]);
}

#[test]
fn test_population_stats() {
    let mut pop = Population::new(42, 10);

    pop.add_initial(Genome::new(), 1.0);
    pop.add_initial(Genome::new(), 2.0);
    pop.add_initial(Genome::new(), 3.0);

    let stats = pop.stats();
    assert_eq!(stats.size, 3);
    assert_eq!(stats.min_fitness, 1.0);
    assert_eq!(stats.max_fitness, 3.0);
    assert!((stats.mean_fitness - 2.0).abs() < 0.001);
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

#[test]
fn test_population_diversity_maintained() {
    // Simulate a steady-state evolution scenario
    let mut pop = Population::new(42, 5);

    // Initial population
    for i in 0..5 {
        let genome = Genome::from_offsets(vec![i as u8]);
        pop.add_initial(genome, i as f32);
    }

    // Try inserting various offspring
    // This tests that diversity is maintained through competition
    for i in 5..20 {
        let genome = Genome::from_offsets(vec![i as u8]);
        let fitness = (i % 7) as f32;
        pop.try_insert(genome, fitness);
    }

    // Should still be at capacity
    assert_eq!(pop.size(), 5);

    // Population should have changed (better individuals survived)
    assert!(pop.best_current().unwrap().fitness >= 4.0);
}

// ============ Evolution Tests ============

#[test]
fn test_evolution_creation() {
    let config = EvolutionConfig {
        population_size: 5,
        max_growth_steps: 10,
        ..Default::default()
    };
    let evo = Evolution::new(42, config);

    assert_eq!(*evo.state(), EvolutionState::Seeding);
    assert_eq!(evo.population().size(), 0);
}

#[test]
fn test_evolution_with_seed() {
    let evo = Evolution::with_seed(42);
    assert_eq!(*evo.state(), EvolutionState::Seeding);
}

#[test]
fn test_evolution_stats_initial() {
    let config = EvolutionConfig {
        population_size: 5,
        ..Default::default()
    };
    let evo = Evolution::new(42, config);
    let stats = evo.stats();

    assert_eq!(stats.generation, 0);
    assert_eq!(stats.population_size, 0);
    assert_eq!(stats.evaluations, 0);
}
