//! Test evolutive (Fasi 40-42): office (docx/xlsx), TOC, tabelle, downsample.

use lopdf::{Dictionary, Document, Object, Stream};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

fn helv(doc: &mut Document) -> lopdf::ObjectId {
    let mut f = Dictionary::new();
    f.set("Type", nome("Font"));
    f.set("Subtype", nome("Type1"));
    f.set("BaseFont", nome("Helvetica"));
    f.set("Encoding", nome("WinAnsiEncoding"));
    doc.add_object(f)
}

fn finalizza(doc: &mut Document, page_id: lopdf::ObjectId, pages_id: lopdf::ObjectId, sr_id: lopdf::ObjectId) -> std::path::PathBuf {
    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", 1);
    doc.set_object(pages_id, pages);
    let mut mi = Dictionary::new();
    mi.set("Marked", true);
    let mut cat = Dictionary::new();
    cat.set("Type", nome("Catalog"));
    cat.set("Pages", Object::Reference(pages_id));
    cat.set("StructTreeRoot", Object::Reference(sr_id));
    cat.set("MarkInfo", Object::Dictionary(mi));
    let cid = doc.add_object(cat);
    doc.trailer.set("Root", Object::Reference(cid));
    let path = std::env::temp_dir().join(format!("pdfa_evo4_{}.pdf", doc.max_id));
    doc.save(&path).unwrap();
    path
}

/// PDF taggato con un titolo H1 "Capitolo 1".
fn crea_h1() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let h1_id = doc.new_object_id();
    let sr_id = doc.new_object_id();
    let font_id = helv(&mut doc);
    let contenuto = b"/H1 <</MCID 0>> BDC BT /F1 18 Tf 72 700 Td (Capitolo 1) Tj ET EMC".to_vec();
    let content_id = doc.add_object(Stream::new(Dictionary::new(), contenuto));

    let mut h1 = Dictionary::new();
    h1.set("Type", nome("StructElem"));
    h1.set("S", nome("H1"));
    h1.set("Pg", Object::Reference(page_id));
    h1.set("K", Object::Integer(0));
    doc.set_object(h1_id, h1);
    let mut sr = Dictionary::new();
    sr.set("Type", nome("StructTreeRoot"));
    sr.set("K", Object::Array(vec![Object::Reference(h1_id)]));
    doc.set_object(sr_id, sr);

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
    finalizza(&mut doc, page_id, pages_id, sr_id)
}

/// PDF taggato con una tabella TR>[TH "Nome", TD "Mario"].
fn crea_tabella() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let table_id = doc.new_object_id();
    let tr_id = doc.new_object_id();
    let th_id = doc.new_object_id();
    let td_id = doc.new_object_id();
    let sr_id = doc.new_object_id();
    let font_id = helv(&mut doc);
    let contenuto = b"/TH <</MCID 0>> BDC BT /F1 12 Tf 72 700 Td (Nome) Tj ET EMC /TD <</MCID 1>> BDC BT /F1 12 Tf 200 700 Td (Mario) Tj ET EMC".to_vec();
    let content_id = doc.add_object(Stream::new(Dictionary::new(), contenuto));

    for (id, s, mcid) in [(th_id, "TH", 0), (td_id, "TD", 1)] {
        let mut c = Dictionary::new();
        c.set("Type", nome("StructElem"));
        c.set("S", nome(s));
        c.set("Pg", Object::Reference(page_id));
        c.set("P", Object::Reference(tr_id));
        c.set("K", Object::Integer(mcid));
        doc.set_object(id, c);
    }
    let mut tr = Dictionary::new();
    tr.set("Type", nome("StructElem"));
    tr.set("S", nome("TR"));
    tr.set("P", Object::Reference(table_id));
    tr.set("K", Object::Array(vec![Object::Reference(th_id), Object::Reference(td_id)]));
    doc.set_object(tr_id, tr);
    let mut table = Dictionary::new();
    table.set("Type", nome("StructElem"));
    table.set("S", nome("Table"));
    table.set("P", Object::Reference(sr_id));
    table.set("K", Object::Reference(tr_id));
    doc.set_object(table_id, table);
    let mut sr = Dictionary::new();
    sr.set("Type", nome("StructTreeRoot"));
    sr.set("K", Object::Reference(table_id));
    doc.set_object(sr_id, sr);

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
    finalizza(&mut doc, page_id, pages_id, sr_id)
}

#[test]
fn toc_antepone_una_pagina() {
    let input = crea_h1();
    let out = std::env::temp_dir().join("pdfa_evo4_toc.pdf");
    let n = pdfa_core::toc::genera(&input, &out).expect("toc");
    assert_eq!(n, 1, "una voce di sommario");
    let doc = lopdf::Document::load(&out).unwrap();
    assert_eq!(doc.get_pages().len(), 2, "pagina indice + originale");
}

#[test]
fn estrae_tabelle_dai_tag() {
    let input = crea_tabella();
    let tab = pdfa_core::lettura::tabelle(&input).expect("tabelle");
    assert_eq!(tab.len(), 1, "una tabella");
    assert_eq!(tab[0].len(), 1, "una riga");
    assert_eq!(tab[0][0], vec!["Nome".to_string(), "Mario".to_string()]);
}

#[test]
fn export_docx_e_xlsx() {
    let h1 = crea_h1();
    let docx = std::env::temp_dir().join("pdfa_evo4.docx");
    pdfa_core::office::pdf_a_docx(&h1, &docx).expect("docx");
    let b = std::fs::read(&docx).unwrap();
    assert_eq!(&b[0..2], b"PK", "docx è uno zip");

    let tab = crea_tabella();
    let xlsx = std::env::temp_dir().join("pdfa_evo4.xlsx");
    pdfa_core::office::pdf_a_xlsx(&tab, &xlsx).expect("xlsx");
    let b2 = std::fs::read(&xlsx).unwrap();
    assert_eq!(&b2[0..2], b"PK", "xlsx è uno zip");
}

#[test]
fn downsample_su_pdf_senza_immagini() {
    let input = crea_h1();
    let out = std::env::temp_dir().join("pdfa_evo4_ds.pdf");
    let (prima, dopo) = pdfa_core::ottimizzazione::comprimi_immagini(&input, &out, 1000, 70).expect("downsample");
    assert!(prima > 0 && dopo > 0);
    // Resta un PDF valido.
    assert!(pdfa_core::analizza(&out).is_ok());
}
