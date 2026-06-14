//! Test della pipeline tag/lettura/validazione/correzione su un PDF taggato
//! minimale costruito al volo con lopdf (non serve Pdfium).

use lopdf::{Dictionary, Document, Object, Stream};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// Crea un PDF taggato: una pagina con un paragrafo (P -> MCID 0 -> "Ciao
/// mondo") e una Figure senza Alt. Ritorna il path.
fn crea_pdf_taggato() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");

    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let p_id = doc.new_object_id();
    let figure_id = doc.new_object_id();
    let struct_root_id = doc.new_object_id();

    // Font Helvetica/WinAnsi per la decodifica del testo.
    let mut font = Dictionary::new();
    font.set("Type", nome("Font"));
    font.set("Subtype", nome("Type1"));
    font.set("BaseFont", nome("Helvetica"));
    font.set("Encoding", nome("WinAnsiEncoding"));
    let font_id = doc.add_object(font);

    // Content stream: testo "Ciao mondo" dentro il marked content MCID 0.
    let contenuto = b"/P <</MCID 0>> BDC BT /F1 12 Tf 72 700 Td (Ciao mondo) Tj ET EMC".to_vec();
    let content_id = doc.add_object(Stream::new(Dictionary::new(), contenuto));

    // Paragrafo P -> MCID 0.
    let mut p = Dictionary::new();
    p.set("Type", nome("StructElem"));
    p.set("S", nome("P"));
    p.set("Pg", Object::Reference(page_id));
    p.set("K", Object::Integer(0));
    doc.set_object(p_id, p);

    // Figure senza Alt.
    let mut figure = Dictionary::new();
    figure.set("Type", nome("StructElem"));
    figure.set("S", nome("Figure"));
    figure.set("Pg", Object::Reference(page_id));
    doc.set_object(figure_id, figure);

    let mut root = Dictionary::new();
    root.set("Type", nome("StructTreeRoot"));
    root.set("K", Object::Array(vec![Object::Reference(p_id), Object::Reference(figure_id)]));
    doc.set_object(struct_root_id, root);

    // Risorse della pagina (font).
    let mut fonts = Dictionary::new();
    fonts.set("F1", Object::Reference(font_id));
    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(fonts));

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(resources));
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
fn struttura_lettura_validazione_correzione() {
    let path = crea_pdf_taggato();

    // 1. Struttura: P e Figure ai primi livelli, con riferimenti.
    let info = pdfa_core::analizza(&path).expect("analizza");
    assert!(info.taggato && info.ha_struct_tree);
    assert_eq!(info.radice.len(), 2);
    assert_eq!(info.radice[0].ruolo, "P");
    assert_eq!(info.radice[1].ruolo, "Figure");
    let rif_p = info.radice[0].riferimento.clone().unwrap();
    let rif_fig = info.radice[1].riferimento.clone().unwrap();

    // 2. Lettura in ordine MCID: il paragrafo deve riportare "Ciao mondo".
    let blocchi = pdfa_core::lettura::blocchi(&path).expect("blocchi");
    assert!(
        blocchi.iter().any(|b| b.ruolo == "P" && b.testo.contains("Ciao mondo")),
        "atteso un blocco P con 'Ciao mondo', trovati: {:?}",
        blocchi
    );

    // 3. Validazione: errore per la figura senza Alt.
    let report = pdfa_core::valida(&path).expect("valida");
    assert!(report.esiti.iter().any(|e| e.regola.contains("alternativo")
        && matches!(e.gravita, pdfa_core::validazione::Gravita::Errore)));

    // 4. Correzione: Alt sulla figura + cambio ruolo P -> H1.
    let corretto = std::env::temp_dir().join("pdfa_test_corretto.pdf");
    let correzioni = pdfa_core::correzione::Correzioni {
        alt: vec![(rif_fig.clone(), "Logo aziendale".into())],
        ruoli: vec![(rif_p.clone(), "H1".into())],
        ..Default::default()
    };
    pdfa_core::correzione::applica(&path, &corretto, &correzioni).expect("correzione");

    let info2 = pdfa_core::analizza(&corretto).expect("analizza dopo");
    assert_eq!(info2.radice[0].ruolo, "H1", "P diventa H1");
    assert_eq!(info2.radice[1].alt.as_deref(), Some("Logo aziendale"));
    let report2 = pdfa_core::valida(&corretto).expect("valida dopo");
    assert!(!report2.esiti.iter().any(|e| e.regola.contains("alternativo")
        && matches!(e.gravita, pdfa_core::validazione::Gravita::Errore)));

    // 5. Riordino: Figure prima di P.
    let riord = std::env::temp_dir().join("pdfa_test_riord.pdf");
    pdfa_core::correzione::riordina(&path, &riord, &[rif_fig, rif_p]).expect("riordina");
    let info3 = pdfa_core::analizza(&riord).expect("analizza riord");
    assert_eq!(info3.radice[0].ruolo, "Figure", "ora la Figure e' prima");
    assert_eq!(info3.radice[1].ruolo, "P");
}
