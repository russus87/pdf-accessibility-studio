// Evita una seconda finestra di console su Windows in modalita' release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pdfa_lib::run()
}
