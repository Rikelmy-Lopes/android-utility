//! Devices reported by the ADB server.

/// Connection state of a device reported by ADB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Ready for commands.
    Online,
    /// Connected but not responding.
    Offline,
    /// Connected but not authorized for debugging.
    Unauthorized,
    /// USB access needs host permission changes.
    NoPermissions,
    /// Unrecognized state reported by ADB.
    Unknown,
}

/// A device reported by the ADB server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Serial or emulator identifier.
    pub serial: String,
    /// Reported connection state.
    pub state: DeviceState,
    /// Device model, when reported.
    pub model: Option<String>,
}

impl Device {
    /// Returns the model name when known, otherwise the serial.
    pub fn name(&self) -> &str {
        match self.model.as_deref() {
            Some(model) if !model.trim().is_empty() => model,
            _ => &self.serial,
        }
    }

    /// Returns `true` when commands can be sent to the device.
    pub fn is_online(&self) -> bool {
        self.state == DeviceState::Online
    }
}
