//! Point d'entrée : assemblage des crates, boucle d'événements, état partagé.

fn main() {
    env_logger::init();
    log::info!("ovl-app {} — squelette de migration", env!("CARGO_PKG_VERSION"));

    // Phase 2 : créer l'overlay, phase 3 : tray + hotkeys, phase 4 : fenêtre egui.
    let _ = ovl_core::amd::cpu_temp();
}
