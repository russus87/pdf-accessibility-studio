// Smoke test manuale di tutta la logica del core.
// Uso: cargo run -p pdfa-core --example smoke -- file_a.pdf [file_b.pdf]
use std::path::{Path, PathBuf};

fn main() {
    let a = std::env::args().nth(1).expect("passa almeno un PDF");
    let b = std::env::args().nth(2);
    pdfa_core::pdfium::inizializza(&[PathBuf::from("pdfium")]).expect("pdfium non caricato");

    let pa = Path::new(&a);
    let info = pdfa_core::apri(pa).expect("apertura");
    println!("[apri] pagine={} titolo={:?}", info.pagine, info.titolo);

    let png = pdfa_core::render_pagina(pa, 0, 600).expect("render");
    println!("[render] {} byte", png.len());

    let testo = pdfa_core::testo_pagine(pa).expect("testo");
    println!("[testo] {} pagine, pag1 {} caratteri", testo.len(), testo.first().map(|s| s.len()).unwrap_or(0));

    let report = pdfa_core::valida(pa).expect("valida");
    println!("[valida] {} errori, {} avvisi:", report.errori, report.avvisi);
    for e in &report.esiti {
        println!("    {:?} - {}", e.gravita, e.regola);
    }

    let segn = pdfa_core::segnalibri::segnalibri(pa).expect("segnalibri");
    println!("[segnalibri] {} voci", segn.len());

    let xml = pdfa_core::export::esporta_xml(pa).expect("xml");
    println!("[export xml] {} caratteri", xml.len());
    let json = pdfa_core::export::esporta_json(pa).expect("json");
    println!("[export json] {} caratteri", json.len());

    // Correzione assistita: applica lingua + titolo + DisplayDocTitle e rivalida.
    let fix = std::env::temp_dir().join("smoke_corretto.pdf");
    let correzioni = pdfa_core::correzione::Correzioni {
        lang: Some("it-IT".into()),
        titolo: Some("Documento Corretto àèì".into()),
        display_doc_title: true,
        ..Default::default()
    };
    pdfa_core::correzione::applica(pa, &fix, &correzioni).expect("correzione");
    let dopo = pdfa_core::valida(&fix).expect("valida dopo");
    println!("[correzione] salvato {:?}; ora {} errori, {} avvisi", fix, dopo.errori, dopo.avvisi);

    if let Some(b) = b {
        let pb = Path::new(&b);
        let dt = pdfa_core::confronto::confronta_testo(pa, pb).expect("confronto testo");
        println!("[confronto testo] uguali={} +{} -{}", dt.uguali, dt.aggiunte, dt.rimosse);
        let dtag = pdfa_core::confronto::confronta_tag(pa, pb).expect("confronto tag");
        println!("[confronto tag] uguali={} +{} -{}", dtag.uguali, dtag.aggiunte, dtag.rimosse);
        let ci = pdfa_core::confronto::confronta_immagine(pa, pb, 0, 500).expect("confronto img");
        println!("[confronto img] {:.2}% diversi ({} px)", ci.percentuale, ci.pixel_diversi);

        let pdf = pdfa_core::confronto::report_pdf("A.pdf", "B.pdf", &dt, &dtag).expect("report pdf");
        std::fs::write("/tmp/report.pdf", &pdf).unwrap();
        println!("[report pdf] {} byte -> /tmp/report.pdf", pdf.len());
    }
}
