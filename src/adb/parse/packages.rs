//! Parsing of `pm list packages` output.

use std::collections::{BTreeMap, HashSet};

use android_utility_core::PackageName;

/// One `pm list packages` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    /// Parsed package name.
    pub package: PackageName,
    /// APK path reported when `-f` was used.
    pub apk_path: Option<String>,
}

/// Parses `pm list packages` output, keeping APK paths when present.
pub fn parse_package_entries(output: &str) -> Vec<PackageEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let remainder = line.strip_prefix("package:")?;

            let (apk_path, raw_package) = match remainder.rsplit_once('=') {
                Some((path, raw_package)) => (Some(path.to_owned()), raw_package),
                None => (None, remainder),
            };

            let package = PackageName::parse(raw_package.trim()).ok()?;

            Some(PackageEntry { package, apk_path })
        })
        .collect()
}

/// Extracts the set of package names from `pm list packages` output.
pub fn parse_package_names(output: &str) -> HashSet<PackageName> {
    parse_package_entries(output)
        .into_iter()
        .map(|entry| entry.package)
        .collect()
}

/// Merges duplicate packages, preferring the base APK path.
pub fn merge_package_paths(entries: &[PackageEntry]) -> BTreeMap<PackageName, Option<String>> {
    let mut merged: BTreeMap<PackageName, Option<String>> = BTreeMap::new();

    for entry in entries {
        let candidate = entry.apk_path.as_deref();
        let preferred = candidate.is_some_and(|path| path.ends_with("/base.apk"));

        match merged.get_mut(&entry.package) {
            Some(existing) => {
                let existing_is_base = existing
                    .as_deref()
                    .is_some_and(|path| path.ends_with("/base.apk"));

                if !existing_is_base && (preferred || existing.is_none()) {
                    *existing = entry.apk_path.clone();
                }
            }
            None => {
                merged.insert(entry.package.clone(), entry.apk_path.clone());
            }
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_entries_with_and_without_paths() {
        let entries = parse_package_entries(
            "package:/data/app/~~abc==/com.example-1/base.apk=com.example\n\
             package:com.other\n\
             not a package line\n\
             package:com.invalid-name\n",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].package.as_str(), "com.example");
        assert_eq!(
            entries[0].apk_path.as_deref(),
            Some("/data/app/~~abc==/com.example-1/base.apk")
        );
        assert_eq!(entries[1].apk_path, None);
    }

    #[test]
    fn merges_duplicate_packages_and_prefers_base_apk() {
        let entries = parse_package_entries(
            "package:/data/app/com.example-1/split_config.arm64.apk=com.example\n\
             package:/data/app/com.example-1/base.apk=com.example\n\
             package:com.plain\n\
             package:com.plain\n",
        );

        let merged = merge_package_paths(&entries);

        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged[&PackageName::parse("com.example").unwrap()].as_deref(),
            Some("/data/app/com.example-1/base.apk")
        );
        assert_eq!(merged[&PackageName::parse("com.plain").unwrap()], None);
    }
}
