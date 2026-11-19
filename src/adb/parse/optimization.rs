//! Parsing of device idle and background execution output.

use std::collections::HashSet;

use android_utility_core::{DozeWhitelistEntry, DozeWhitelistSource, PackageName};

/// Parses `dumpsys deviceidle whitelist` output.
///
/// Lines are reported as `source,package`, but older devices may print the
/// package name alone. Unrecognized lines are ignored.
pub fn parse_doze_whitelist(output: &str) -> Vec<DozeWhitelistEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() {
                return None;
            }

            let (source, raw_package) = match line.split_once(',') {
                Some((source, package)) => (
                    doze_whitelist_source(source),
                    package.split(',').next().unwrap_or(package).trim(),
                ),
                None => (DozeWhitelistSource::Unknown, line),
            };

            let package = PackageName::parse(raw_package).ok()?;

            Some(DozeWhitelistEntry { package, source })
        })
        .collect()
}

/// Parses `cmd appops query-op RUN_ANY_IN_BACKGROUND ignore` output.
///
/// The command prints one package name per line. Unrecognized lines are
/// ignored so that vendor diagnostics do not break the listing.
pub fn parse_ignored_background_packages(output: &str) -> HashSet<PackageName> {
    output
        .lines()
        .filter_map(|line| PackageName::parse(line.trim()).ok())
        .collect()
}

fn doze_whitelist_source(raw: &str) -> DozeWhitelistSource {
    match raw.trim().to_ascii_lowercase().as_str() {
        "system" | "system-excidle" | "system-ui" => DozeWhitelistSource::System,
        "user" => DozeWhitelistSource::User,
        _ => DozeWhitelistSource::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ignored_background_packages() {
        let packages = parse_ignored_background_packages(
            "com.example.chat\n\
             com.example.mail\n\
             \n\
             Error: something unexpected\n\
             com.invalid-name\n",
        );

        assert_eq!(packages.len(), 2);
        assert!(packages.contains(&PackageName::parse("com.example.chat").unwrap()));
        assert!(packages.contains(&PackageName::parse("com.example.mail").unwrap()));
        assert!(parse_ignored_background_packages("").is_empty());
    }

    #[test]
    fn parses_doze_whitelist_sources() {
        let entries = parse_doze_whitelist(
            "system,com.android.shell\n\
             user,com.example.chat\n\
             system-excidle,com.google.android.gms\n\
             com.example.legacy\n\
             \n\
             not a package line\n\
             user,com.invalid-name\n",
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].package.as_str(), "com.android.shell");
        assert_eq!(entries[0].source, DozeWhitelistSource::System);
        assert_eq!(entries[1].package.as_str(), "com.example.chat");
        assert_eq!(entries[1].source, DozeWhitelistSource::User);
        assert_eq!(
            entries[2].source,
            DozeWhitelistSource::System,
            "temporary system exemptions count as system entries"
        );
        assert_eq!(entries[3].package.as_str(), "com.example.legacy");
        assert_eq!(entries[3].source, DozeWhitelistSource::Unknown);
    }

    #[test]
    fn ignores_empty_or_unrecognized_doze_whitelist_output() {
        assert!(parse_doze_whitelist("").is_empty());
        assert!(parse_doze_whitelist("Whitelist:\n").is_empty());
    }

    #[test]
    fn parses_doze_whitelist_lines_with_a_trailing_user_id() {
        let entries = parse_doze_whitelist(
            "system-excidle,com.android.shell,2000\n\
             system,com.android.shell,2000\n\
             user,com.whatsapp,10305\n\
             user,com.example,not-a-user-id\n",
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].package.as_str(), "com.android.shell");
        assert_eq!(entries[0].source, DozeWhitelistSource::System);
        assert_eq!(entries[2].package.as_str(), "com.whatsapp");
        assert_eq!(entries[2].source, DozeWhitelistSource::User);
    }
}
