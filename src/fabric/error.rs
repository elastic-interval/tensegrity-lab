use std::fmt;

/// Error types that can occur in the fabric module
#[derive(Debug, Clone)]
pub enum FabricError {
    /// Error when trying to access attachment points on a non-push interval
    NotPushInterval,
    /// Error when trying to access an attachment point with an invalid index
    InvalidAttachmentIndex,
    /// Error when trying to access an interval that doesn't exist
    IntervalNotFound,
    /// Error when trying to access a joint that doesn't exist
    JointNotFound,
    /// Error when trying to perform an operation that requires valid joints
    InvalidJointIndices,
}

impl fmt::Display for FabricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FabricError::NotPushInterval => write!(f, "Not a push interval"),
            FabricError::InvalidAttachmentIndex => write!(f, "Invalid attachment point index"),
            FabricError::IntervalNotFound => write!(f, "Interval not found"),
            FabricError::JointNotFound => write!(f, "Joint not found"),
            FabricError::InvalidJointIndices => write!(f, "Invalid joint indices"),
        }
    }
}

impl std::error::Error for FabricError {}
