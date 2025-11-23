/// Integration test to verify Rust DSL brick definitions

use tensegrity_lab::build::dsl::brick_builders::build_brick_library;
use tensegrity_lab::build::dsl::brick_library::BrickLibrary;

#[test]
fn test_brick_library_construction() {
    // Build the brick library using Rust DSL
    let brick_library = BrickLibrary::new(build_brick_library());

    // Verify we got the expected number of bricks
    assert_eq!(brick_library.brick_definitions.len(), 7, "Should have 7 bricks");

    // Verify each brick has the expected structure
    let brick_names = ["Single", "Single", "Omni", "Torque", "TorqueRight", "TorqueLeft", "Equals"];
    for (i, expected_name) in brick_names.iter().enumerate() {
        let brick = &brick_library.brick_definitions[i];
        assert!(
            brick.proto.alias.0.contains(*expected_name),
            "Brick {} should be {}, got {:?}",
            i,
            expected_name,
            brick.proto.alias
        );
    }

    // Verify baked_bricks were computed
    assert_eq!(
        brick_library.baked_bricks.len(),
        112,
        "Should have 112 baked brick variants"
    );

    println!("✓ BrickLibrary constructed successfully!");
    println!("  {} brick definitions", brick_library.brick_definitions.len());
    println!("  {} baked variants", brick_library.baked_bricks.len());
}

#[test]
fn test_individual_bricks() {
    use tensegrity_lab::build::dsl::brick_builders::*;

    // Test each brick function individually
    let single_r = single_right();
    assert!(single_r.proto.alias.0.contains("Single"));
    assert_eq!(single_r.proto.pushes.len(), 3);
    assert_eq!(single_r.proto.pulls.len(), 3);
    assert!(single_r.baked.is_some());

    let single_l = single_left();
    assert!(single_l.proto.alias.0.contains("Single"));
    assert!(single_l.baked.is_some());

    let omni_brick = omni();
    assert!(omni_brick.proto.alias.0.contains("Omni"));
    assert_eq!(omni_brick.proto.pushes.len(), 6);
    assert!(omni_brick.baked.is_some());

    let torque_brick = torque();
    assert!(torque_brick.proto.alias.0.contains("Torque"));
    assert_eq!(torque_brick.proto.pushes.len(), 9);
    assert!(torque_brick.baked.is_some());

    let torque_r = torque_right();
    assert!(torque_r.proto.alias.0.contains("TorqueRight"));
    assert!(torque_r.baked.is_some());

    let torque_l = torque_left();
    assert!(torque_l.proto.alias.0.contains("TorqueLeft"));
    assert!(torque_l.baked.is_some());

    let equals_brick = equals();
    assert!(equals_brick.proto.alias.0.contains("Equals"));
    // Equals doesn't have baked data
    assert!(equals_brick.baked.is_none());

    println!("✓ All individual brick functions work correctly!");
}
