use std::fmt;

/// Error returned when a circular type dependency is detected.
///
/// Contains the full cycle path, e.g. `["A", "B", "C", "A"]`, where the first
/// and last elements are the same type name.
#[derive(Debug, Clone, PartialEq)]
pub struct CircularReference(Vec<String>);

impl CircularReference {
    pub(crate) fn new(cycle: Vec<String>) -> Self {
        Self(cycle)
    }

    /// The sequence of type names forming the cycle.
    pub fn cycle(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Display for CircularReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Circular reference detected: {}", self.0.join(" -> "))
    }
}

impl std::error::Error for CircularReference {}
