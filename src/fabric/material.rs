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

    pub fn spring_constant_at_1m(&self) -> NewtonsPerMeter {
        NewtonsPerMeter(match self {
            Push => 2e10,
            Pull => 6.7e9, // ratio ~3:1 to match aluminum/dyneema
            Spring => 9e4,
        })
    }

    /// Realistic spring constant for physical force calculations
    /// Based on actual material properties (not simulation stiffness)
    pub fn real_spring_constant_at_1m(&self) -> NewtonsPerMeter {
        NewtonsPerMeter(match self {
            // 50mm aluminum tube, 2mm wall: E=70GPa, A≈300mm², k=EA/L≈2.1e7 N/m
            Push => 2.1e7,
            // 14mm Dyneema: E=100GPa, A≈154mm², k=EA/L≈1.5e7 N/m
            Pull => 1.5e7,
            Spring => 9e4,
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
