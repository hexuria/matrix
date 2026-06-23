use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Emitted by `matrix` when a matrix fills all slots and cycles.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatrixCycled {
    /// The owner of the matrix that cycled.
    pub account_id: Uuid,
    /// The id of the fresh matrix spawned by the cycle.
    pub matrix_id: Uuid,
}
