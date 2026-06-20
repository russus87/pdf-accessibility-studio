//! Test della creazione di campi modulo editabili (Fase 26).

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF di una pagina vuota A4 (senza AcroForm).
fn crea_pdf_vuoto() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]));
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

    let path = std::env::temp_dir().join("pdfa_test_campi_in.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn aggiunge_campo_testo_leggibile() {
    let input = crea_pdf_vuoto();
    let out = std::env::temp_dir().join("pdfa_test_campi_out.pdf");

    let campi = vec![pdfa_core::campi::NuovoCampo {
        pagina: 0,
        x_mm: 20.0,
        y_mm: 30.0,
        larghezza_mm: 80.0,
        altezza_mm: 8.0,
        nome: "email".into(),
        tooltip: "Indirizzo email".into(),
        valore: String::new(),
    }];
    let n = pdfa_core::campi::aggiungi(&input, &out, &campi).expect("aggiungi");
    assert_eq!(n, 1);

    // Il campo deve risultare leggibile dal lettore AcroForm (Fase 20).
    let letti = pdfa_core::form::leggi(&out).expect("leggi form");
    assert_eq!(letti.len(), 1, "un campo creato");
    assert_eq!(letti[0].nome.as_deref(), Some("email"));
    assert_eq!(letti[0].tipo.as_deref(), Some("Tx"));
    assert_eq!(letti[0].tooltip.as_deref(), Some("Indirizzo email"));
    assert_eq!(letti[0].pagina, Some(0));
}
