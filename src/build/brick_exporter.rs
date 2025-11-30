#![cfg(not(target_arch = "wasm32"))]

use crate::build::dsl::brick_dsl::BrickName;

const BAKED_BRICKS_PATH: &str = "src/build/dsl/brick_library/baked_bricks.rs";

pub fn export(brick_name: BrickName, baked_code: &str) {
    let source = std::fs::read_to_string(BAKED_BRICKS_PATH)
        .expect("Failed to read baked_bricks.rs");
    let func_name = function_name(brick_name);
    let new_source = substitute_baked_section(&source, func_name, baked_code)
        .expect(&format!("Failed to find {} in baked_bricks.rs", func_name));
    std::fs::write(BAKED_BRICKS_PATH, &new_source)
        .expect("Failed to write baked_bricks.rs");
}

fn function_name(brick_name: BrickName) -> &'static str {
    match brick_name {
        BrickName::SingleTwistLeft => "single_twist_left_baked",
        BrickName::SingleTwistRight => "single_twist_right_baked",
        BrickName::OmniSymmetrical => "omni_symmetrical_baked",
        BrickName::OmniTetrahedral => "omni_tetrahedral_baked",
        BrickName::TorqueSymmetrical => "torque_symmetrical_baked",
    }
}

fn substitute_baked_section(
    source: &str,
    func_name: &str,
    replacement: &str,
) -> Option<String> {
    let func_start = source.find(&format!("fn {}()", func_name))?;
    let after_func = &source[func_start..];
    let scale_offset = after_func.find("scale:")?;
    let scale_start = func_start + scale_offset;

    // Back up to start of line (after newline)
    let line_start = source[..scale_start].rfind('\n').map(|i| i + 1).unwrap_or(0);

    let after_scale = &source[scale_start..];
    let faces_offset = after_scale.find("faces:")?;
    let faces_start = scale_start + faces_offset;

    let mut new_source = String::with_capacity(source.len());
    new_source.push_str(&source[..line_start]);
    new_source.push_str(replacement);
    new_source.push_str("\n        ");
    new_source.push_str(&source[faces_start..]);

    Some(new_source)
}
