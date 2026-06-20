//! Test della marcatura Artifact (Fase 18) su un PDF taggato minimale.

use lopdf::{Dictionary, Document, Object, Stream};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF taggato: una pagina con due paragrafi (P0 -> MCID 0 -> "Titolo pagina";
/// P1 -> MCID 1 -> "Corpo del testo"). Marcheremo P0 (finto header) come artifact.
fn crea_pdf() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");

    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let p0_id = doc.new_object_id();
    let p1_id = doc.new_object_id();
    let struct_root_id = doc.new_object_id();

    let mut font = Dictionary::new();
    font.set("Type", nome("Font"));
    font.set("Subtype", nome("Type1"));
    font.set("BaseFont", nome("Helvetica"));
    font.set("Encoding", nome("WinAnsiEncoding"));
    let font_id = doc.add_object(font);

    let contenuto = b"/P <</MCID 0>> BDC BT /F1 12 Tf 72 750 Td (Titolo pagina) Tj ET EMC \
                      /P <</MCID 1>> BDC BT /F1 12 Tf 72 700 Td (Corpo del testo) Tj ET EMC"
        .to_vec();
    let content_id = doc.add_object(Stream::new(Dictionary::new(), contenuto));

    for (id, mcid) in [(p0_id, 0), (p1_id, 1)] {
        let mut p = Dictionary::new();
        p.set("Type", nome("StructElem"));
        p.set("S", nome("P"));
        p.set("Pg", Object::Reference(page_id));
        p.set("K", Object::Integer(mcid));
        doc.set_object(id, p);
    }

    let mut root = Dictionary::new();
    root.set("Type", nome("StructTreeRoot"));
    root.set("K", Object::Array(vec![Object::Reference(p0_id), Object::Reference(p1_id)]));
    doc.set_object(struct_root_id, root);

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
    catalog.set("Lang", Object::string_literal("it-IT"));
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join("pdfa_test_artifact.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn marca_artifact_stacca_e_riscrive() {
    let path = crea_pdf();

    // Stato iniziale: due P, "Titolo pagina" e "Corpo del testo" nella lettura.
    let info = pdfa_core::analizza(&path).expect("analizza");
    assert_eq!(info.radice.len(), 2);
    let rif_p0 = info.radice[0].riferimento.clone().unwrap();

    // Marca il primo paragrafo (finto header) come artifact.
    let out = std::env::temp_dir().join("pdfa_test_artifact_out.pdf");
    let n = pdfa_core::artifact::marca_artifact(&path, &out, &[rif_p0]).expect("marca");
    assert_eq!(n, 1);

    // Dopo: nell'albero resta solo il secondo P.
    let info2 = pdfa_core::analizza(&out).expect("analizza dopo");
    assert_eq!(info2.radice.len(), 1, "un solo elemento resta nell'albero");
    assert_eq!(info2.radice[0].ruolo, "P");

    // La lettura logica non contiene piu' il titolo (ora artifact), ma il corpo si'.
    let blocchi = pdfa_core::lettura::blocchi(&out).expect("blocchi");
    assert!(
        !blocchi.iter().any(|b| b.testo.contains("Titolo pagina")),
        "il titolo non deve piu' essere nell'ordine di lettura: {blocchi:?}"
    );
    assert!(
        blocchi.iter().any(|b| b.testo.contains("Corpo del testo")),
        "il corpo deve restare leggibile: {blocchi:?}"
    );

    // Il content stream deve contenere /Artifact e non piu' l'MCID 0 taggato come /P.
    let doc = lopdf::Document::load(&out).unwrap();
    let pages: Vec<_> = doc.get_pages().into_iter().collect();
    let dati = doc.get_page_content(pages[0].1).unwrap();
    let testo = String::from_utf8_lossy(&dati);
    assert!(testo.contains("Artifact"), "atteso /Artifact nel contenuto: {testo}");
}
