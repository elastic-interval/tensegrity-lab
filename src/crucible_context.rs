use crate::crucible::Stage;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{LabEvent, Radio};

/// Provides controlled access to Crucible's state during iteration
pub struct CrucibleContext<'a> {
    pub fabric: &'a mut Fabric,
    pub physics: &'a mut Physics,
    pub radio: &'a Radio,
    transition_stage: Option<Stage>,
    events: Vec<LabEvent>,
}

impl<'a> CrucibleContext<'a> {
    pub fn new(fabric: &'a mut Fabric, physics: &'a mut Physics, radio: &'a Radio) -> Self {
        Self {
            fabric,
            physics,
            radio,
            transition_stage: None,
            events: Vec::new(),
        }
    }

    /// Replace the current fabric with a new one
    pub fn replace_fabric(&mut self, new_fabric: Fabric) {
        *self.fabric = new_fabric;
    }

    /// Replace the current physics with new physics
    pub fn replace_physics(&mut self, new_physics: Physics) {
        *self.physics = new_physics;
    }

    /// Request a transition to a new stage
    pub fn transition_to(&mut self, new_stage: Stage) {
        self.transition_stage = Some(new_stage);
    }

    /// Queue an event to be sent
    pub fn queue_event(&mut self, event: LabEvent) {
        self.events.push(event);
    }

    /// Send an event immediately
    pub fn send_event(&self, event: LabEvent) {
        event.send(self.radio);
    }

    /// Apply all queued changes and return the requested stage transition
    pub fn apply_changes(self) -> Option<Stage> {
        // Send all queued events
        for event in self.events {
            event.send(self.radio);
        }

        // Return the stage transition
        self.transition_stage
    }
}
