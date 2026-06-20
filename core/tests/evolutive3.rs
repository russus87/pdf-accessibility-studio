//! Test evolutive gestione (Fasi 35-38): mail merge, annotazioni, ritaglia/inserisci.

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

fn crea_vuoto(tag: &str) -> std::path::PathBuf {
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
    let cid = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(cid));
    let path = std::env::temp_dir().join(format!("pdfa_evo3_{tag}.pdf"));
    doc.save(&path).unwrap();
    path
}

#[test]
fn mail_merge_un_pdf_per_record() {
    let pdfs = pdfa_core::modello::genera_unione(
        "<html><body><h1>Ciao {{nome}}</h1></body></html>",
        r#"[{"nome":"Anna"},{"nome":"Luca"},{"nome":"Sara"}]"#,
    )
    .expect("unione");
    assert_eq!(pdfs.len(), 3, "un PDF per record");
    for p in &pdfs {
        assert_eq!(&p[0..4], b"%PDF");
    }
}

#[test]
fn annotazioni_applica_e_riepilogo() {
    let input = crea_vuoto("annot");
    let out = std::env::temp_dir().join("pdfa_evo3_annot_out.pdf");
    let annot = vec![
        pdfa_core::annotazioni::Annot::Evidenzia {
            pagina: 0, x_mm: 20.0, y_mm: 30.0, larghezza_mm: 60.0, altezza_mm: 6.0,
            colore: pdfa_core::annotazioni::Colore { r: 255, g: 235, b: 60 },
        },
        pdfa_core::annotazioni::Annot::Nota {
            pagina: 0, x_mm: 100.0, y_mm: 30.0, contenuto: "Da rivedere".into(),
            colore: pdfa_core::annotazioni::Colore { r: 255, g: 200, b: 0 },
        },
    ];
    let n = pdfa_core::annotazioni::applica(&input, &out, &annot).expect("annota");
    assert_eq!(n, 2);

    let r = pdfa_core::annotazioni::riepilogo(&out).expect("riepilogo");
    assert_eq!(r.len(), 2);
    assert!(r.iter().any(|v| v.tipo == "Highlight"), "evidenziazione presente: {r:?}");
    assert!(r.iter().any(|v| v.tipo == "Text" && v.contenuto == "Da rivedere"), "nota presente: {r:?}");
}

#[test]
fn ritaglia_imposta_cropbox() {
    let input = crea_vuoto("crop");
    let out = std::env::temp_dir().join("pdfa_evo3_crop_out.pdf");
    pdfa_core::pagine::ritaglia(&input, &out, &[1], 10.0, 10.0, 100.0, 150.0).expect("ritaglia");
    let doc = lopdf::Document::load(&out).unwrap();
    let page_id = doc.get_pages().into_values().next().unwrap();
    let d = doc.get_object(page_id).unwrap().as_dict().unwrap();
    assert!(d.get(b"CropBox").is_ok(), "CropBox impostato");
}

#[test]
fn inserisci_pagine_da_altro_file() {
    let base = crea_vuoto("base");
    let altro = crea_vuoto("altro");
    let out = std::env::temp_dir().join("pdfa_evo3_insert_out.pdf");
    pdfa_core::pagine::inserisci(&base, &altro, 1, &out).expect("inserisci");
    let doc = lopdf::Document::load(&out).unwrap();
    assert_eq!(doc.get_pages().len(), 2, "ora ci sono due pagine");
}
