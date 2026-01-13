use crate::fabric::{IntervalKey, JointKey};
use crate::Age;

/// Lifecycle state of a growing Push interval.
///
/// All timing is based on fabric age, not iteration counts.
#[derive(Clone, Debug)]
pub enum PushState {
    /// Just spawned, connected to parent by 1/3-2/3 pulls, seeking additional connections.
    /// Can pivot freely around the parent endpoint until secondary connections are made.
    Pivoting {
        /// The parent endpoint this Push is connected to
        parent_endpoint: JointKey,
        /// Fabric age when this Push started pivoting (for timeout calculation)
        start_age: Age,
        /// Fabric age of last sensing attempt
        last_sense_age: Age,
    },

    /// Has sufficient connections, stable orientation, can spawn children.
    Anchored {
        /// Whether the alpha endpoint has already spawned a child
        alpha_spawned: bool,
        /// Whether the omega endpoint has already spawned a child
        omega_spawned: bool,
        /// Fabric age when this Push became anchored (for spawn delay calculation)
        anchored_age: Age,
    },

    /// Failed to anchor in time, being removed from the fabric.
    Dying,
}

/// A Push interval participating in the growth process.
///
/// Each GrowingPush tracks its lifecycle state and connections separately from
/// the Fabric's interval data, enabling growth-specific behavior.
#[derive(Clone, Debug)]
pub struct GrowingPush {
    /// The fabric interval key for this Push
    pub interval_key: IntervalKey,
    /// The alpha (first) joint of this Push
    pub alpha_key: JointKey,
    /// The omega (second) joint of this Push
    pub omega_key: JointKey,
    /// Current lifecycle state
    pub state: PushState,
    /// Pull intervals connecting this Push to its parent (the 1/3 and 2/3 pulls)
    pub parent_pulls: Vec<IntervalKey>,
    /// Secondary pull connections to other Pushes (anchor connections)
    pub anchor_pulls: Vec<IntervalKey>,
}

impl GrowingPush {
    /// Create a new GrowingPush in Pivoting state.
    pub fn new_pivoting(
        interval_key: IntervalKey,
        alpha_key: JointKey,
        omega_key: JointKey,
        parent_endpoint: JointKey,
        parent_pulls: Vec<IntervalKey>,
        current_age: Age,
    ) -> Self {
        Self {
            interval_key,
            alpha_key,
            omega_key,
            state: PushState::Pivoting {
                parent_endpoint,
                start_age: current_age,
                last_sense_age: current_age,
            },
            parent_pulls,
            anchor_pulls: Vec::new(),
        }
    }

    /// Create a new GrowingPush in Anchored state (for seed Push).
    pub fn new_anchored(
        interval_key: IntervalKey,
        alpha_key: JointKey,
        omega_key: JointKey,
        current_age: Age,
    ) -> Self {
        Self {
            interval_key,
            alpha_key,
            omega_key,
            state: PushState::Anchored {
                alpha_spawned: false,
                omega_spawned: false,
                anchored_age: current_age,
            },
            parent_pulls: Vec::new(),
            anchor_pulls: Vec::new(),
        }
    }

    /// Check if this Push is in Pivoting state.
    pub fn is_pivoting(&self) -> bool {
        matches!(self.state, PushState::Pivoting { .. })
    }

    /// Check if this Push is in Anchored state.
    pub fn is_anchored(&self) -> bool {
        matches!(self.state, PushState::Anchored { .. })
    }

    /// Check if this Push is dying.
    pub fn is_dying(&self) -> bool {
        matches!(self.state, PushState::Dying)
    }

    /// Check if the given endpoint can spawn a new Push.
    pub fn can_spawn(&self, endpoint: JointKey, current_age: Age, spawn_delay: crate::units::Seconds) -> bool {
        match &self.state {
            PushState::Anchored {
                alpha_spawned,
                omega_spawned,
                anchored_age,
            } => {
                // Check if enough time has passed since anchoring
                let elapsed = current_age.elapsed_since(*anchored_age);
                if elapsed.0 < spawn_delay.0 {
                    return false;
                }
                if endpoint == self.alpha_key && !alpha_spawned {
                    return true;
                }
                if endpoint == self.omega_key && !omega_spawned {
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Mark an endpoint as having spawned.
    pub fn mark_spawned(&mut self, endpoint: JointKey) {
        if let PushState::Anchored {
            alpha_spawned,
            omega_spawned,
            ..
        } = &mut self.state
        {
            if endpoint == self.alpha_key {
                *alpha_spawned = true;
            } else if endpoint == self.omega_key {
                *omega_spawned = true;
            }
        }
    }

    /// Get the endpoint that is NOT the parent (for Pivoting pushes).
    pub fn free_endpoint(&self) -> Option<JointKey> {
        match &self.state {
            PushState::Pivoting { parent_endpoint, .. } => {
                if self.alpha_key == *parent_endpoint {
                    Some(self.omega_key)
                } else {
                    Some(self.alpha_key)
                }
            }
            _ => None,
        }
    }

    /// Transition from Pivoting to Anchored.
    pub fn anchor(&mut self, current_age: Age) {
        if self.is_pivoting() {
            self.state = PushState::Anchored {
                alpha_spawned: false,
                omega_spawned: false,
                anchored_age: current_age,
            };
        }
    }

    /// Transition to Dying state.
    pub fn die(&mut self) {
        self.state = PushState::Dying;
    }

    /// Add an anchor pull connection.
    pub fn add_anchor_pull(&mut self, pull_key: IntervalKey) {
        self.anchor_pulls.push(pull_key);
    }

    /// Get count of anchor pulls (secondary connections).
    pub fn anchor_pull_count(&self) -> usize {
        self.anchor_pulls.len()
    }

    /// Get all interval keys associated with this Push (for removal).
    pub fn all_interval_keys(&self) -> Vec<IntervalKey> {
        let mut keys = vec![self.interval_key];
        keys.extend(self.parent_pulls.iter().copied());
        keys.extend(self.anchor_pulls.iter().copied());
        keys
    }
}
