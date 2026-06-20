//! Test della generazione della bozza di albero dei tag (opzione A) a partire
//! da un PDF *non* taggato. Solo lopdf, niente Pdfium.

use lopdf::{Dictionary, Document, Object};

use pdfa_core::correzione::{genera_struttura, Elemento};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// Crea un PDF di 2 pagine vuote, senza struttura, e lo salva.
fn crea_non_taggato() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let mut kids = Vec::new();
    for _ in 0..2 {
        let pid = doc.new_object_id();
        let mut page = Dictionary::new();
        page.set("Type", nome("Page"));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        doc.set_object(pid, page);
        kids.push(Object::Reference(pid));
    }

    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Count", 2_i64);
    pages.set("Kids", Object::Array(kids));
    doc.set_object(pages_id, pages);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    let cid = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(cid));

    let path = std::env::temp_dir().join("pdfa_genera_origine.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn bozza_da_pdf_non_taggato() {
    let origine = crea_non_taggato();

    // Partenza: non taggato.
    let prima = pdfa_core::analizza(&origine).unwrap();
    assert!(!prima.taggato);
    assert!(!prima.ha_struct_tree);

    let elementi = vec![
        Elemento { ruolo: "H1".into(), pagina: Some(0), alt: None },
        Elemento { ruolo: "P".into(), pagina: Some(0), alt: None },
        Elemento { ruolo: "Figure".into(), pagina: Some(1), alt: Some("Un grafico".into()) },
    ];

    let dest = std::env::temp_dir().join("pdfa_genera_bozza.pdf");
    let n = genera_struttura(&origine, &dest, Some("it"), Some("Documento di prova"), &elementi).unwrap();
    assert_eq!(n, 3);

    // La bozza deve essere un PDF valido e taggato.
    let dopo = pdfa_core::analizza(&dest).unwrap();
    assert!(dopo.taggato, "MarkInfo/Marked deve essere true");
    assert!(dopo.ha_struct_tree, "deve avere uno StructTreeRoot");
    assert_eq!(dopo.lang.as_deref(), Some("it"));
    assert_eq!(dopo.titolo.as_deref(), Some("Documento di prova"));

    // L'albero deve contenere i ruoli proposti (sotto l'elemento Document).
    let ruoli: Vec<&str> = dopo.radice.iter().map(|n| n.ruolo.as_str()).collect();
    // La radice raccolta e' il "Document"; i figli sono H1/P/Figure.
    let figli_doc: Vec<&str> = dopo
        .radice
        .iter()
        .flat_map(|n| n.figli.iter())
        .map(|n| n.ruolo.as_str())
        .collect();
    assert!(
        ruoli.contains(&"Document") || figli_doc.contains(&"H1"),
        "deve esserci l'elemento Document o i suoi figli"
    );
    assert!(figli_doc.contains(&"Figure"), "la figura deve essere presente");

    // La figura deve avere l'Alt impostato.
    let alt_figura = dopo
        .radice
        .iter()
        .flat_map(|n| n.figli.iter())
        .find(|n| n.ruolo == "Figure")
        .and_then(|n| n.alt.clone());
    assert_eq!(alt_figura.as_deref(), Some("Un grafico"));
}
