use crate::build::tenscript::brick_library::BrickLibrary;
use crate::crucible::Stage;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{LabEvent, Radio};

/// Provides controlled access to Crucible's state during iteration
pub struct CrucibleContext<'a> {
    pub fabric: &'a mut Fabric,
    pub physics: &'a mut Physics,
    pub radio: &'a Radio,
    pub brick_library: &'a BrickLibrary,
    transition_stage: Option<Stage>,
    events: Vec<LabEvent>,
    camera_translation: Option<cgmath::Vector3<f32>>,
}

impl<'a> CrucibleContext<'a> {
    pub fn new(
        fabric: &'a mut Fabric,
        physics: &'a mut Physics,
        radio: &'a Radio,
        brick_library: &'a BrickLibrary,
    ) -> Self {
        Self {
            fabric,
            physics,
            radio,
            brick_library,
            transition_stage: None,
            events: Vec::new(),
            camera_translation: None,
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

    /// Set a camera translation to be applied synchronously
    pub fn set_camera_translation(&mut self, translation: cgmath::Vector3<f32>) {
        self.camera_translation = Some(translation);
    }

    /// Apply all queued changes and return the requested stage transition and camera translation
    pub fn apply_changes(self) -> (Option<Stage>, Option<cgmath::Vector3<f32>>) {
        // Send all queued events
        for event in self.events {
            event.send(self.radio);
        }

        // Return the stage transition and camera translation
        (self.transition_stage, self.camera_translation)
    }
}
