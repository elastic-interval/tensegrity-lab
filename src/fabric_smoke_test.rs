#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use strum::IntoEnumIterator;

    /// Every named fabric should build to completion. Catches regressions
    /// in brick attachment, hub composition, and shape-step execution.
    #[test]
    fn test_all_fabrics_build() {
        for fabric_name in FabricName::iter() {
            let plan = fabric_library::get_fabric_plan(fabric_name);
            let mut executor = FabricPlanExecutor::new(plan);
            while *executor.stage() == ExecutorStage::Building {
                let _ = executor.iterate();
            }
            let fabric = &executor.fabric;
            assert!(
                !fabric.joints.is_empty(),
                "{fabric_name}: built with zero joints"
            );
            assert!(
                !fabric.intervals.is_empty(),
                "{fabric_name}: built with zero intervals"
            );
        }
    }
}
