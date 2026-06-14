// Smoke test manuale: carica Pdfium da ./pdfium, apre un PDF e renderizza pag.0.
// Uso: cargo run -p pdfa-core --example smoke -- /percorso/file.pdf
use std::path::PathBuf;
fn main() {
    let arg = std::env::args().nth(1).expect("passa il percorso di un PDF");
    pdfa_core::pdfium::inizializza(&[PathBuf::from("pdfium")]).expect("pdfium non caricato");
    let info = pdfa_core::apri(std::path::Path::new(&arg)).expect("apertura fallita");
    println!("pagine={} titolo={:?}", info.pagine, info.titolo);
    let png = pdfa_core::render_pagina(std::path::Path::new(&arg), 0, 800).expect("render fallito");
    std::fs::write("/tmp/smoke_out.png", &png).unwrap();
    println!("PNG scritto: {} byte -> /tmp/smoke_out.png", png.len());
}
