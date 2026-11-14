use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut sampler = ovl_core::Sampler::new();
    sampler.sample(); // amorce PDH (deux points de mesure nécessaires)
    for _ in 0..3 {
        sleep(Duration::from_secs(1));
        let stats = sampler.sample();
        println!("{}", serde_json::to_string_pretty(&stats).unwrap_or_default());
    }
}
