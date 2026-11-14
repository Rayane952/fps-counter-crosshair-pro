//! Charge CPU / fréquence / RAM via PDH.

/// TODO(phase 1) : `PdhOpenQuery` + `\\Processor Information(_Total)\\% Processor Utility`.
pub fn load() -> Option<f32> {
    None
}

/// TODO(phase 1) : `GlobalMemoryStatusEx`.
pub fn memory() -> (u64, u64) {
    (0, 0)
}
