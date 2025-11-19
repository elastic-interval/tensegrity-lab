use crate::build::tenscript::FabricPlan;

#[derive(Clone, Default, Debug)]
pub struct FabricLibrary {
    pub fabric_plans: Vec<FabricPlan>,
}

impl FabricLibrary {
    /// Create a FabricLibrary from fabric plans.
    ///
    /// # Example
    /// ```
    /// use tensegrity_lab::build::fabric_builders::build_fabric_library;
    /// use tensegrity_lab::build::tenscript::fabric_library::FabricLibrary;
    ///
    /// let fabric_library = FabricLibrary::new(build_fabric_library());
    /// ```
    pub fn new(fabric_plans: Vec<FabricPlan>) -> Self {
        FabricLibrary { fabric_plans }
    }

    pub fn fabric_list(&self) -> Vec<String> {
        self.fabric_plans
            .iter()
            .map(|plan| plan.name.clone())
            .collect()
    }
}
