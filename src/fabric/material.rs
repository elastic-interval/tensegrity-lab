use crate::fabric::interval::Role;
use crate::fabric::material::Material::{Pull, Push, Spring};
use crate::fabric::physics::Physics;
use crate::units::{Grams, GramsPerMeter, Meters, NewtonsPerMeter, Unit};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Push,
    Pull,
    Spring,
}

impl Material {
    fn base_linear_density(&self) -> GramsPerMeter {
        GramsPerMeter(match self {
            Push => 1500.0,   // 1.5 kg/m: aluminum tube ~50mm diameter
            Pull => 50.0,     // Dyneema rope ~10mm diameter
            Spring => 1000.0, // 1 kg/m: steel coil spring
        })
    }

    pub fn linear_density(&self, physics: &Physics) -> GramsPerMeter {
        GramsPerMeter(self.base_linear_density().0 * physics.mass_multiplier())
    }

    /// Calculate mass from length in meters
    pub fn mass(&self, length: Meters, physics: &Physics) -> Grams {
        self.linear_density(physics) * length
    }

    fn spring_constant_at_1m(&self) -> NewtonsPerMeter {
        NewtonsPerMeter(match self {
            Push => 2e10,   // aluminum tube ~50mm diameter
            Pull => 2e9,    // Dyneema rope ~10mm diameter
            Spring => 9e4,  // steel coil spring for actuation
        })
    }

    /// Spring constant for a given length (in meters)
    /// k(L) = k(1m) / L
    pub fn spring_constant(&self, length: Meters, physics: &Physics) -> NewtonsPerMeter {
        let k_at_1m = self.spring_constant_at_1m();
        let k = (k_at_1m.f32() / length.f32().max(0.001)) * physics.rigidity_multiplier();
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
