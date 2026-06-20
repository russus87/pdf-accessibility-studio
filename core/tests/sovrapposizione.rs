//! Test della sovrapposizione (Fase 24): applica una filigrana testuale a un PDF
//! e verifica che il testo sia poi presente/estraibile. Richiede libpdfium.

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF minimale di una pagina vuota A4.
fn crea_pdf_vuoto() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]));
    page.set("Contents", Object::Array(vec![]));
    page.set("Resources", Object::Dictionary(Dictionary::new()));
    doc.set_object(page_id, page);

    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", 1);
    doc.set_object(pages_id, pages);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join("pdfa_test_overlay_in.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn filigrana_testo_finisce_nel_pdf() {
    // Inizializza Pdfium (cartelle vuote -> libreria di sistema /usr/lib/libpdfium.so).
    if pdfa_core::pdfium::inizializza(&[]).is_err() {
        eprintln!("Pdfium non disponibile: salto il test");
        return;
    }

    let input = crea_pdf_vuoto();
    let out = std::env::temp_dir().join("pdfa_test_overlay_out.pdf");

    let filigrana = pdfa_core::sovrapposizione::Filigrana::Testo {
        testo: "RISERVATO".into(),
        dim_pt: 48.0,
        colore: pdfa_core::sovrapposizione::Colore { r: 200, g: 0, b: 0 },
        opacita: 100,
        rotazione: 0.0,
    };
    let elementi = vec![pdfa_core::sovrapposizione::Elemento::Testo {
        pagina: 0,
        x_mm: 20.0,
        y_mm: 20.0,
        testo: "Bozza interna".into(),
        dim_pt: 12.0,
        colore: pdfa_core::sovrapposizione::Colore::default(),
        opacita: 255,
        rotazione: 0.0,
    }];

    let n = pdfa_core::sovrapposizione::applica(&input, &out, &elementi, Some(&filigrana))
        .expect("applica overlay");
    assert_eq!(n, 2, "un elemento testo + una filigrana");

    // Il testo deve risultare estraibile dal PDF salvato.
    let testo = pdfa_core::testo_pagine(&out).expect("estrai testo").join(" ");
    assert!(testo.contains("RISERVATO"), "filigrana presente: {testo:?}");
    assert!(testo.contains("Bozza interna"), "testo presente: {testo:?}");
}
