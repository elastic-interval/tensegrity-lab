// Temporary: collisions after omitting C02.6–C01.1 and D02.6–D01.1.
#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;

    #[test]
    fn hinge_probe() {
        let mut ex = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !ex.is_complete() {
            let _ = ex.iterate();
        }
        ex.fabric.update_all_attachment_connections();
        let f = &ex.fabric;
        let c = f.connector.as_ref().unwrap();
        let (lo, hi) = f.altitude_range();
        let gone = ["AL02.6", "AR02.6"].iter().all(|a| {
            let (x, y) = (f.joint_key_by_label(a).unwrap(), f.joint_key_by_label(&a.replace("02.6", "01.1")).unwrap());
            f.joining((x, y)).is_none()
        });
        for label in ["AL02.6", "AL01.1"] {
            let matches: Vec<_> = f.joints.keys().filter(|&k| f.joint_label(k) == label).collect();
            for jk in &matches {
                let others: Vec<String> = f.intervals.values().filter(|i| i.touches(*jk)).map(|i| format!("{}({:?})", f.joint_label(i.other_joint(*jk)), i.role)).collect();
                eprintln!("{label} ({} joints with this label): {}", matches.len(), others.join(" "));
            }
        }
        eprintln!("settled: height {:.2} m, finite {}, both omitted: {gone}, culprit ends {}", hi - lo, f.joints.values().all(|j| j.location.is_finite()), c.culprits.len());
    }
}
