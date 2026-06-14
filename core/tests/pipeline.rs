//! Test della pipeline tag/validazione/correzione su un PDF taggato minimale
//! costruito al volo con lopdf (non serve Pdfium).

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// Crea un PDF taggato con una pagina e una Figure senza Alt; ritorna il path.
fn crea_pdf_taggato() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");

    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let figure_id = doc.new_object_id();
    let struct_root_id = doc.new_object_id();

    let mut figure = Dictionary::new();
    figure.set("Type", nome("StructElem"));
    figure.set("S", nome("Figure"));
    figure.set("Pg", Object::Reference(page_id));
    doc.set_object(figure_id, figure);

    let mut root = Dictionary::new();
    root.set("Type", nome("StructTreeRoot"));
    root.set("K", Object::Reference(figure_id));
    doc.set_object(struct_root_id, root);

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set(
        "MediaBox",
        Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]),
    );
    doc.set_object(page_id, page);

    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", 1);
    doc.set_object(pages_id, pages);

    let mut markinfo = Dictionary::new();
    markinfo.set("Marked", true);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    catalog.set("StructTreeRoot", Object::Reference(struct_root_id));
    catalog.set("MarkInfo", Object::Dictionary(markinfo));
    catalog.set("Lang", Object::string_literal("en-US"));
    let catalog_id = doc.add_object(catalog);

    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join("pdfa_test_taggato.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn struttura_validazione_e_alt() {
    let path = crea_pdf_taggato();

    // 1. La struttura riconosce il documento taggato e la Figure con riferimento.
    let info = pdfa_core::analizza(&path).expect("analizza");
    assert!(info.taggato, "deve risultare taggato");
    assert!(info.ha_struct_tree);
    assert_eq!(info.radice.len(), 1, "una sola radice (la Figure)");
    let fig = &info.radice[0];
    assert_eq!(fig.ruolo, "Figure");
    assert_eq!(fig.pagina, Some(0), "la Figure punta a pagina 0");
    let riferimento = fig.riferimento.clone().expect("la Figure deve avere un riferimento");
    assert!(fig.alt.is_none(), "all'inizio niente Alt");

    // 2. La validazione segnala la figura senza Alt.
    let report = pdfa_core::valida(&path).expect("valida");
    assert!(
        report.esiti.iter().any(|e| e.regola.contains("alternativo")
            && matches!(e.gravita, pdfa_core::validazione::Gravita::Errore)),
        "deve esserci un errore sull'alt mancante"
    );

    // 3. Applichiamo l'Alt su quel riferimento e salviamo una copia.
    let corretto = std::env::temp_dir().join("pdfa_test_corretto.pdf");
    let correzioni = pdfa_core::correzione::Correzioni {
        alt: vec![(riferimento, "Logo aziendale".to_string())],
        ..Default::default()
    };
    pdfa_core::correzione::applica(&path, &corretto, &correzioni).expect("correzione");

    // 4. Ora la Figure ha l'Alt e la validazione non segnala piu' l'errore.
    let info2 = pdfa_core::analizza(&corretto).expect("analizza dopo");
    assert_eq!(info2.radice[0].alt.as_deref(), Some("Logo aziendale"));
    let report2 = pdfa_core::valida(&corretto).expect("valida dopo");
    assert!(
        !report2.esiti.iter().any(|e| e.regola.contains("alternativo")
            && matches!(e.gravita, pdfa_core::validazione::Gravita::Errore)),
        "l'errore sull'alt deve sparire"
    );
}
