//! Pure domain types shared by the diagnostic core.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationClass {
    ReadOnly,
    VolatileControl,
    ServiceRoutine,
    PersistentChange,
    ForbiddenProgramming,
}

#[cfg(test)]
mod tests {
    use super::OperationClass;

    #[test]
    fn operation_classes_are_explicit() {
        assert_ne!(OperationClass::ReadOnly, OperationClass::PersistentChange);
    }
}
