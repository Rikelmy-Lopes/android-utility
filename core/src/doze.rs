//! Doze whitelist domain models.
//!
//! A package on the Device Idle whitelist is exempt from Doze restrictions.

use crate::app::PackageName;

/// Origin of a Device Idle whitelist entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DozeWhitelistSource {
    /// Added by the system image.
    System,
    /// Added by the user or an application at runtime.
    User,
    /// Origin not reported or not recognized.
    Unknown,
}

/// One package reported by `dumpsys deviceidle whitelist`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DozeWhitelistEntry {
    /// Whitelisted package.
    pub package: PackageName,
    /// Where the entry came from.
    pub source: DozeWhitelistSource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_keeps_package_and_source() {
        let entry = DozeWhitelistEntry {
            package: PackageName::parse("com.example").unwrap(),
            source: DozeWhitelistSource::User,
        };

        assert_eq!(entry.package.as_str(), "com.example");
        assert_eq!(entry.source, DozeWhitelistSource::User);
    }
}
