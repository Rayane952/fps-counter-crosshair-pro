//! GPU : température et ventilateur (pilote, via D3DKMT), mémoire vidéo dédiée (DXGI).
//!
//! La température vient de `D3DKMTQueryAdapterInfo(KMTQAITYPE_ADAPTERPERFDATA)` : c'est la valeur
//! que publie le pilote graphique (AMD Adrenalin ici), celle du Gestionnaire des tâches.
//! Aucun droit administrateur requis.

use std::sync::OnceLock;

use libloading::Library;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
};

const KMTQAITYPE_ADAPTERPERFDATA: i32 = 62;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Luid {
    low: u32,
    high: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AdapterInfo {
    h_adapter: u32,
    luid: Luid,
    num_of_sources: u32,
    precise_present_regions_preferred: i32,
}

#[repr(C)]
struct EnumAdapters2 {
    num_adapters: u32,
    adapters: *mut AdapterInfo,
}

#[repr(C)]
struct QueryAdapterInfo {
    h_adapter: u32,
    kind: i32,
    private_data: *mut core::ffi::c_void,
    private_data_size: u32,
}

#[repr(C)]
struct CloseAdapter {
    h_adapter: u32,
}

/// D3DKMT_ADAPTER_PERFDATA (WDDM 2.4+).
#[repr(C)]
#[derive(Default)]
struct AdapterPerfData {
    physical_adapter_index: u32,
    memory_frequency: u64,
    max_memory_frequency: u64,
    max_memory_frequency_oc: u64,
    memory_bandwidth: u64,
    pcie_bandwidth: u64,
    fan_rpm: u32,
    power: u32,
    /// Dixièmes de degré Celsius.
    temperature: u32,
    power_state_override: u8,
}

type EnumFn = unsafe extern "system" fn(*mut EnumAdapters2) -> i32;
type QueryFn = unsafe extern "system" fn(*mut QueryAdapterInfo) -> i32;
type CloseFn = unsafe extern "system" fn(*mut CloseAdapter) -> i32;

struct Kmt {
    _lib: Library,
    enumerate: EnumFn,
    query: QueryFn,
    close: CloseFn,
}

fn kmt() -> Option<&'static Kmt> {
    static KMT: OnceLock<Option<Kmt>> = OnceLock::new();
    KMT.get_or_init(|| unsafe {
        let lib = Library::new("gdi32.dll").ok()?;
        let enumerate = *lib.get::<EnumFn>(b"D3DKMTEnumAdapters2\0").ok()?;
        let query = *lib.get::<QueryFn>(b"D3DKMTQueryAdapterInfo\0").ok()?;
        let close = *lib.get::<CloseFn>(b"D3DKMTCloseAdapter\0").ok()?;
        Some(Kmt {
            _lib: lib,
            enumerate,
            query,
            close,
        })
    })
    .as_ref()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Sensors {
    pub temperature_c: Option<f32>,
    pub fan_rpm: Option<u32>,
}

/// Capteurs de l'adaptateur le plus chaud (les adaptateurs sans capteur sont ignorés).
pub fn sensors() -> Sensors {
    let Some(kmt) = kmt() else {
        return Sensors::default();
    };
    let mut best = Sensors::default();
    unsafe {
        let mut e = EnumAdapters2 {
            num_adapters: 0,
            adapters: std::ptr::null_mut(),
        };
        if (kmt.enumerate)(&mut e) != 0 || e.num_adapters == 0 {
            return best;
        }
        let mut adapters = vec![AdapterInfo::default(); e.num_adapters as usize];
        e.adapters = adapters.as_mut_ptr();
        if (kmt.enumerate)(&mut e) != 0 {
            return best;
        }
        for adapter in adapters.iter().take(e.num_adapters as usize) {
            let mut perf = AdapterPerfData::default();
            let mut q = QueryAdapterInfo {
                h_adapter: adapter.h_adapter,
                kind: KMTQAITYPE_ADAPTERPERFDATA,
                private_data: &mut perf as *mut _ as *mut _,
                private_data_size: std::mem::size_of::<AdapterPerfData>() as u32,
            };
            if (kmt.query)(&mut q) == 0 && perf.temperature > 0 {
                let t = perf.temperature as f32 / 10.0;
                if t < 150.0 && best.temperature_c.map_or(true, |b| t > b) {
                    best.temperature_c = Some(t);
                    best.fan_rpm = (perf.fan_rpm > 0).then_some(perf.fan_rpm);
                }
            }
            let mut c = CloseAdapter {
                h_adapter: adapter.h_adapter,
            };
            (kmt.close)(&mut c);
        }
    }
    best
}

/// Mémoire vidéo dédiée de l'adaptateur principal (Go).
pub fn vram_total_gb() -> Option<f32> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            index += 1;
            let desc = adapter.GetDesc1().ok()?;
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let bytes = desc.DedicatedVideoMemory as f32;
            if bytes > 0.0 {
                return Some(bytes / 1_073_741_824.0);
            }
        }
        None
    }
}

/// Clé d'un moteur : adaptateur + type (`luid_..._engtype_3D`). La charge GPU est celle du
/// moteur le plus occupé, chaque moteur cumulant tous les processus.
pub fn engine_key(instance: &str) -> String {
    match (
        instance.find("luid_"),
        instance.find("_eng_"),
        instance.find("engtype_"),
    ) {
        (Some(l), Some(e), Some(t)) if l < e => format!("{}_{}", &instance[l..e], &instance[t..]),
        _ => instance.to_owned(),
    }
}
