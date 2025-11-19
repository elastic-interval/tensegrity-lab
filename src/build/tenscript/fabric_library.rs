use crate::build::tenscript::FabricPlan;

#[derive(Clone, Default, Debug)]
pub struct FabricLibrary {
    pub fabric_plans: Vec<FabricPlan>,
}

impl FabricLibrary {
    /// Create FabricLibrary from Rust DSL fabric definitions
    ///
    /// This uses type-safe Rust builders instead of parsing tenscript.
    ///
    /// # Example
    /// ```
    /// use tensegrity_lab::build::fabric_builders::build_fabric_library;
    /// use tensegrity_lab::build::tenscript::fabric_library::FabricLibrary;
    ///
    /// let fabric_library = FabricLibrary::from_rust(build_fabric_library());
    /// ```
    pub fn from_rust(fabric_plans: Vec<FabricPlan>) -> Self {
        FabricLibrary { fabric_plans }
    }

    pub fn fabric_list(&self) -> Vec<String> {
        self.fabric_plans
            .iter()
            .map(|plan| plan.name.clone())
            .collect()
    }
}
