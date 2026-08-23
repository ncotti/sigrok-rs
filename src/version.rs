//! Version module

use libsigrok_sys::sigrok as sr;
use std::fmt;

/// Version information, in the format of <major>.<minor>.<patch>
#[derive(Debug, Clone, Copy, Default)]
pub struct Version {
    /// Version major number.
    pub major: u32,
    /// Version minor number.
    pub minor: u32,
    /// Version patch number.
    pub patch: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Returns libsigrok library version
pub fn get_sr_lib_version() -> Version {
    Version {
        major: unsafe { sr::sr_lib_version_current_get() as u32 },
        minor: unsafe { sr::sr_lib_version_revision_get() as u32 },
        patch: unsafe { sr::sr_lib_version_age_get() as u32 },
    }
}

/// Returns sigrok package version, i.e., the version of the binary as
/// downloaded from the package manager.
pub fn get_sr_package_version() -> Version {
    Version {
        major: unsafe { sr::sr_package_version_major_get() as u32 },
        minor: unsafe { sr::sr_package_version_minor_get() as u32 },
        patch: unsafe { sr::sr_package_version_micro_get() as u32 },
    }
}
