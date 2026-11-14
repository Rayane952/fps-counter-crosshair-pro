//! Charge GPU, VRAM, température via D3DKMT (données de performance de l'adaptateur).

/// TODO(phase 1) : `D3DKMTQueryAdapterInfo(KMTQAITYPE_ADAPTERPERFDATA)`.
pub fn load() -> Option<f32> {
    None
}

pub fn temperature() -> Option<f32> {
    None
}
