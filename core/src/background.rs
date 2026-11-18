//! Background execution policy of an installed application.
//!
//! The policy is read from the `RUN_ANY_IN_BACKGROUND` application operation.

/// Background execution policy reported by `cmd appops get`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundExecution {
    /// The application may run in the background.
    Allowed,
    /// The application is restricted from running in the background.
    Ignored,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_variants_are_distinct() {
        assert_ne!(BackgroundExecution::Allowed, BackgroundExecution::Ignored);
    }
}
