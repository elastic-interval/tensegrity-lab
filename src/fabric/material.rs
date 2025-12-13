use crate::fabric::interval::Role;
use crate::fabric::material::Material::{Pull, Push, Spring};
use crate::fabric::physics::Physics;
use crate::units::{Grams, GramsPerMeter, Meters, NewtonsPerMeter};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Push,
    Pull,
    Spring,
}

impl Material {
    /// Base linear density: mass per unit length (g/m)
    fn base_linear_density(&self) -> GramsPerMeter {
        GramsPerMeter(match self {
            // 1500 g/m (1.5 kg/m) - heavy compression strut
            Push => 1500.0,
            // 50 g/m - light cable
            Pull => 50.0,
            // 1000 g/m (1 kg/m) - medium spring
            Spring => 1000.0,
        })
    }

    pub fn linear_density(&self, physics: &Physics) -> GramsPerMeter {
        GramsPerMeter(self.base_linear_density().0 * physics.mass_multiplier())
    }

    /// Calculate mass from length in meters
    pub fn mass(&self, length: Meters, physics: &Physics) -> Grams {
        self.linear_density(physics) * length
    }

    /// Spring constant at 1m reference length
    /// k × L = constant (spring constant inversely proportional to length)
    ///
    /// Push: 2×10¹⁰ N/m at 1m - comparable to a 50mm diameter aluminum tube
    /// Pull: 2×10⁹ N/m at 1m - comparable to 10mm diameter Dyneema rope
    fn spring_constant_at_1m(&self) -> NewtonsPerMeter {
        NewtonsPerMeter(match self {
            Push => 2e10,
            Pull => 2e9,
            Spring => 9.0e4,
        })
    }

    /// Spring constant for a given length (in meters)
    /// k(L) = k(1m) / L
    pub fn spring_constant(&self, length: Meters, physics: &Physics) -> NewtonsPerMeter {
        let k_at_1m = self.spring_constant_at_1m();
        let k = (*k_at_1m / (*length).max(0.001)) * physics.rigidity_multiplier();
        NewtonsPerMeter(k)
    }

    pub fn default_role(&self) -> Role {
        match self {
            Push => Role::Pushing,
            Pull => Role::Pulling,
            Spring => Role::Springy,
        }
    }
}
