//! Test evolutive (Fasi 31-34, parte lopdf): ottimizzazione, estrazione, compila moduli.

use lopdf::{Dictionary, Document, Object, Stream};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF taggato: P -> MCID 0 -> "Ciao mondo".
fn crea_taggato() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let p_id = doc.new_object_id();
    let sr_id = doc.new_object_id();

    let mut font = Dictionary::new();
    font.set("Type", nome("Font"));
    font.set("Subtype", nome("Type1"));
    font.set("BaseFont", nome("Helvetica"));
    font.set("Encoding", nome("WinAnsiEncoding"));
    let font_id = doc.add_object(font);

    let contenuto = b"/P <</MCID 0>> BDC BT /F1 12 Tf 72 700 Td (Ciao mondo) Tj ET EMC".to_vec();
    let content_id = doc.add_object(Stream::new(Dictionary::new(), contenuto));

    let mut p = Dictionary::new();
    p.set("Type", nome("StructElem"));
    p.set("S", nome("P"));
    p.set("Pg", Object::Reference(page_id));
    p.set("K", Object::Integer(0));
    doc.set_object(p_id, p);

    let mut root = Dictionary::new();
    root.set("Type", nome("StructTreeRoot"));
    root.set("K", Object::Array(vec![Object::Reference(p_id)]));
    doc.set_object(sr_id, root);

    let mut fonts = Dictionary::new();
    fonts.set("F1", Object::Reference(font_id));
    let mut res = Dictionary::new();
    res.set("Font", Object::Dictionary(fonts));

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(res));
    doc.set_object(page_id, page);

    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", 1);
    doc.set_object(pages_id, pages);

    let mut mi = Dictionary::new();
    mi.set("Marked", true);
    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    catalog.set("StructTreeRoot", Object::Reference(sr_id));
    catalog.set("MarkInfo", Object::Dictionary(mi));
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join("pdfa_evo2_taggato.pdf");
    doc.save(&path).unwrap();
    path
}

/// PDF con un campo testo AcroForm.
fn crea_form() -> (std::path::PathBuf, String) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let field_id = doc.new_object_id();

    let mut field = Dictionary::new();
    field.set("FT", nome("Tx"));
    field.set("T", Object::string_literal("nome"));
    field.set("Subtype", nome("Widget"));
    field.set("Rect", Object::Array(vec![100.into(), 600.into(), 300.into(), 620.into()]));
    field.set("P", Object::Reference(page_id));
    doc.set_object(field_id, field);

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]));
    page.set("Annots", Object::Array(vec![Object::Reference(field_id)]));
    doc.set_object(page_id, page);

    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", 1);
    doc.set_object(pages_id, pages);

    let mut acro = Dictionary::new();
    acro.set("Fields", Object::Array(vec![Object::Reference(field_id)]));
    let acro_id = doc.add_object(acro);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    catalog.set("AcroForm", Object::Reference(acro_id));
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join("pdfa_evo2_form.pdf");
    doc.save(&path).unwrap();
    (path, format!("{}_{}", field_id.0, field_id.1))
}

#[test]
fn ottimizzazione_produce_pdf_valido() {
    let input = crea_taggato();
    let out = std::env::temp_dir().join("pdfa_evo2_ottim.pdf");
    let (prima, dopo) = pdfa_core::ottimizzazione::ottimizza(&input, &out).expect("ottimizza");
    assert!(prima > 0 && dopo > 0);
    // L'output deve essere un PDF valido e ancora taggato.
    let info = pdfa_core::analizza(&out).expect("analizza ottimizzato");
    assert!(info.ha_struct_tree, "i tag restano dopo l'ottimizzazione");
}

#[test]
fn estrazione_html_e_markdown() {
    let input = crea_taggato();
    let html = pdfa_core::estrazione::esporta(&input, pdfa_core::estrazione::Formato::Html).expect("html");
    assert!(html.contains("Ciao mondo"), "testo nell'HTML: {html}");
    assert!(html.contains("<p>"), "paragrafo nell'HTML");
    let md = pdfa_core::estrazione::esporta(&input, pdfa_core::estrazione::Formato::Markdown).expect("md");
    assert!(md.contains("Ciao mondo"), "testo nel Markdown");
}

#[test]
fn compila_modulo_imposta_valore() {
    let (input, rif) = crea_form();
    let out = std::env::temp_dir().join("pdfa_evo2_compilato.pdf");
    let valori = vec![(rif.clone(), "Mario Rossi".to_string())];
    let n = pdfa_core::form::compila(&input, &out, &valori, false).expect("compila");
    assert_eq!(n, 1);
    let campi = pdfa_core::form::leggi(&out).expect("leggi");
    assert_eq!(campi[0].valore.as_deref(), Some("Mario Rossi"));
}
