//! Test evolutive (Fasi 43-45): auto-remediation, sanitizzazione, editor segnalibri.

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF di N pagine vuote A4, con catalogo extra (callback) per i test.
fn crea(n: usize, extra: impl FnOnce(&mut Document, &mut Dictionary)) -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for _ in 0..n {
        let pid = doc.new_object_id();
        let mut page = Dictionary::new();
        page.set("Type", nome("Page"));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]));
        doc.set_object(pid, page);
        kids.push(Object::Reference(pid));
    }
    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Count", n as i64);
    pages.set("Kids", Object::Array(kids));
    doc.set_object(pages_id, pages);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    extra(&mut doc, &mut catalog);
    let cid = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(cid));

    let path = std::env::temp_dir().join(format!("pdfa_evo5_{}.pdf", doc.max_id));
    doc.save(&path).unwrap();
    path
}

#[test]
fn sanitizza_rimuove_js_e_metadati() {
    let input = crea(1, |doc, cat| {
        // OpenAction JavaScript.
        let mut oa = Dictionary::new();
        oa.set("S", nome("JavaScript"));
        oa.set("JS", Object::string_literal("app.alert('ciao')"));
        cat.set("OpenAction", Object::Dictionary(oa));
        // Names/JavaScript.
        let mut names = Dictionary::new();
        names.set("JavaScript", Object::Dictionary(Dictionary::new()));
        cat.set("Names", Object::Dictionary(names));
        // Info con autore.
        let mut info = Dictionary::new();
        info.set("Author", Object::string_literal("Mario"));
        let info_id = doc.add_object(info);
        doc.trailer.set("Info", Object::Reference(info_id));
    });

    let out = std::env::temp_dir().join("pdfa_evo5_san.pdf");
    let esito = pdfa_core::sanitizza::pulisci(&input, &out, &Default::default()).expect("sanitizza");
    assert!(esito.javascript && esito.metadati);

    let doc = lopdf::Document::load(&out).unwrap();
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let cat = doc.get_object(root).unwrap().as_dict().unwrap();
    assert!(cat.get(b"OpenAction").is_err(), "OpenAction rimosso");
    let info_id = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    let info = doc.get_object(info_id).unwrap().as_dict().unwrap();
    assert!(info.get(b"Author").is_err(), "autore rimosso");
}

#[test]
fn editor_segnalibri_scrive_outline() {
    let input = crea(3, |_, _| {});
    let out = std::env::temp_dir().join("pdfa_evo5_bm.pdf");
    let voci = vec![
        ("Introduzione".to_string(), 1u8, 0i32),
        ("Dettagli".to_string(), 2u8, 1i32),
        ("Conclusione".to_string(), 1u8, 2i32),
    ];
    let n = pdfa_core::segnalibri::imposta(&input, &out, &voci).expect("imposta");
    assert_eq!(n, 3);

    let doc = lopdf::Document::load(&out).unwrap();
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let cat = doc.get_object(root).unwrap().as_dict().unwrap();
    assert!(cat.get(b"Outlines").is_ok(), "catalogo ha /Outlines");
}

#[test]
fn auto_remediation_riduce_errori() {
    // PDF taggato ma senza /Lang né titolo: la validazione segnala errori.
    let input = crea(1, |doc, cat| {
        let sr = doc.new_object_id();
        let mut p = Dictionary::new();
        p.set("Type", nome("StructElem"));
        p.set("S", nome("P"));
        let p_id = doc.add_object(p);
        let mut root = Dictionary::new();
        root.set("Type", nome("StructTreeRoot"));
        root.set("K", Object::Array(vec![Object::Reference(p_id)]));
        doc.set_object(sr, root);
        let mut mi = Dictionary::new();
        mi.set("Marked", true);
        cat.set("MarkInfo", Object::Dictionary(mi));
        cat.set("StructTreeRoot", Object::Reference(sr));
    });

    let out = std::env::temp_dir().join("pdfa_evo5_auto.pdf");
    let (prima, dopo) = pdfa_core::correzione::auto(&input, &out, "it-IT", "Documento di prova").expect("auto");
    assert!(dopo < prima, "gli errori devono diminuire: {prima} -> {dopo}");
}

#[test]
fn matterhorn_report() {
    // PDF taggato senza /Lang: il report deve avere almeno un checkpoint fallito.
    let input = crea(1, |doc, cat| {
        let sr = doc.new_object_id();
        let mut p = Dictionary::new();
        p.set("Type", nome("StructElem"));
        p.set("S", nome("P"));
        let p_id = doc.add_object(p);
        let mut root = Dictionary::new();
        root.set("Type", nome("StructTreeRoot"));
        root.set("K", Object::Array(vec![Object::Reference(p_id)]));
        doc.set_object(sr, root);
        let mut mi = Dictionary::new();
        mi.set("Marked", true);
        cat.set("MarkInfo", Object::Dictionary(mi));
        cat.set("StructTreeRoot", Object::Reference(sr));
    });

    let r = pdfa_core::matterhorn::analizza(&input).expect("matterhorn");
    assert!(r.voci.len() > 20, "molti checkpoint");
    assert!(r.falliti >= 1, "lingua mancante => almeno un fallito");
    assert!(r.manuali >= 1, "alcuni checkpoint sono manuali");
    let html = pdfa_core::matterhorn::report_html("test", &r);
    assert!(html.contains("Matterhorn") && html.contains("WCAG"));
}
