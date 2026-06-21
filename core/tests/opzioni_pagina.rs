#[test]
fn genera_con_opzioni_header_footer_numeri() {
    let opz = pdfa_core::modello::OpzioniPagina {
        larghezza_mm: Some(210.0),
        altezza_mm: Some(297.0),
        margine_alto: Some(25.0),
        margine_basso: Some(20.0),
        margine_sx: Some(18.0),
        margine_dx: Some(18.0),
        intestazione: Some("Documento riservato".into()),
        pie_pagina: Some("Azienda S.p.A.".into()),
        numeri_pagina: Some(true),
        salta_prima: Some(true),
        ..Default::default()
    };
    let pdf = pdfa_core::modello::genera_pdf_opz(
        pdfa_core::modello::modello_esempio(),
        pdfa_core::modello::dati_esempio(),
        &opz,
    ).expect("genera con opzioni");
    assert_eq!(&pdf[0..4], b"%PDF");
    assert!(pdf.len() > 100);
}

#[test]
fn colonne_e_posizione_assoluta_non_crashano() {
    let html = r#"<!doctype html><html><head></head><body>
      <div style="position:absolute;top:20mm;left:20mm;width:50mm">Pannello</div>
      <div style="column-count:2;column-gap:10mm"><p>uno due tre quattro cinque sei sette otto nove dieci</p></div>
    </body></html>"#;
    let pdf = pdfa_core::modello::html_a_pdf(html).expect("colonne+absolute");
    assert_eq!(&pdf[0..4], b"%PDF");
}

#[test]
fn impaginato_con_header_footer_e_numeri() {
    // Pdfium serve per rasterizzare e sovrapporre le bande. Se non è caricabile
    // (ambiente senza libreria) il test si limita a non fallire.
    let _ = pdfa_core::pdfium::inizializza(&[
        std::path::PathBuf::from("../pdfium"),
        std::path::PathBuf::from("../src-tauri/pdfium"),
        std::path::PathBuf::from("pdfium"),
    ]);
    if pdfa_core::pdfium::istanza().is_err() {
        eprintln!("Pdfium non disponibile: test saltato");
        return;
    }
    let dir = std::env::temp_dir();
    let dest = dir.join("pdfa_test_impaginato.pdf");
    // Nuovo modello a frammenti: corpo/header/footer sono frammenti HTML, lo
    // stile è condiviso.
    let body = "<p>Riga di prova ripetuta per riempire più pagine.</p>".repeat(120);
    let header = r#"<div style="font:12px sans-serif">Documento riservato</div>"#;
    let footer = r#"<div style="font:11px sans-serif;text-align:center">Pag. {{PAGENUM}} di {{TTLPAGES}}</div>"#;
    let opz = pdfa_core::modello::OpzioniPagina {
        larghezza_mm: Some(210.0), altezza_mm: Some(297.0),
        margine_alto: Some(15.0), margine_basso: Some(15.0),
        margine_sx: Some(18.0), margine_dx: Some(18.0),
        header_mm: Some(18.0), footer_mm: Some(14.0),
        ..Default::default()
    };
    let n = pdfa_core::modello::genera_impaginato(&dest, &body, "{}", header, footer, "body{font-family:sans-serif}", &opz)
        .expect("impaginato");
    assert!(n >= 2, "il corpo deve produrre più pagine, n={n}");
    let bytes = std::fs::read(&dest).unwrap();
    assert_eq!(&bytes[0..4], b"%PDF");
    let _ = std::fs::remove_file(&dest);
}
