//! Parsing of `adb devices -l` output.

use super::super::DeviceState;

/// Parsed representation of one line from `adb devices -l`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    /// Device serial or emulator identifier.
    pub serial: String,
    /// Reported connection state.
    pub state: DeviceState,
    /// Device model reported by the transport, when present.
    pub model: Option<String>,
}

/// Parses `adb devices -l` output.
pub fn parse_devices(output: &str) -> Vec<DeviceEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
                return None;
            }

            let serial = line.split_whitespace().next()?.to_owned();
            let lowercase_line = line.to_ascii_lowercase();

            let state = if lowercase_line.contains("no permissions") {
                DeviceState::NoPermissions
            } else {
                match line.split_whitespace().nth(1)? {
                    "device" => DeviceState::Online,
                    "offline" => DeviceState::Offline,
                    "unauthorized" => DeviceState::Unauthorized,
                    _ => DeviceState::Unknown,
                }
            };

            let model = line
                .split_whitespace()
                .find_map(|token| token.strip_prefix("model:").map(str::to_owned));

            Some(DeviceEntry {
                serial,
                state,
                model,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_devices() {
        let devices = parse_devices(
            "List of devices attached\n\
             emulator-5554\tdevice product:sdk model:Pixel_7 device:x transport_id:1\n\
             R58M12345\toffline\n\
             ABC123\tunauthorized\n\
             XYZ\tno permissions (user in plugdev group); see [http://developer.android.com]\n",
        );

        assert_eq!(devices.len(), 4);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert_eq!(devices[0].state, DeviceState::Online);
        assert_eq!(devices[0].model.as_deref(), Some("Pixel_7"));
        assert_eq!(devices[1].state, DeviceState::Offline);
        assert_eq!(devices[2].state, DeviceState::Unauthorized);
        assert_eq!(devices[3].state, DeviceState::NoPermissions);
    }

    #[test]
    fn ignores_device_list_header_and_empty_output() {
        assert!(parse_devices("").is_empty());
        assert!(parse_devices("List of devices attached\n").is_empty());
    }
}
