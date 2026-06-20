//! Test delle operazioni sulle pagine (senza Pdfium, solo lopdf).

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// Crea un PDF con `n` pagine vuote e lo salva con suffisso `tag`.
fn crea(n: usize, tag: &str) -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let mut kids = Vec::new();
    for _ in 0..n {
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
    pages.set("Count", n as i64);
    pages.set("Kids", Object::Array(kids));
    doc.set_object(pages_id, pages);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    let cid = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(cid));

    let path = std::env::temp_dir().join(format!("pdfa_pagine_{tag}.pdf"));
    doc.save(&path).unwrap();
    path
}

fn n_pagine(p: &std::path::Path) -> usize {
    Document::load(p).unwrap().get_pages().len()
}

#[test]
fn elimina_estrai_riordina_unisci_ruota() {
    let tre = crea(3, "src");

    // Elimina la pagina 2 -> restano 2.
    let out = std::env::temp_dir().join("pdfa_pagine_del.pdf");
    pdfa_core::pagine::elimina(&tre, &out, &[2]).unwrap();
    assert_eq!(n_pagine(&out), 2);

    // Estrai [1, 3] -> 2 pagine.
    let ext = std::env::temp_dir().join("pdfa_pagine_ext.pdf");
    pdfa_core::pagine::estrai(&tre, &ext, &[1, 3]).unwrap();
    assert_eq!(n_pagine(&ext), 2);

    // Riordina [3, 2, 1] -> ancora 3 pagine.
    let rio = std::env::temp_dir().join("pdfa_pagine_rio.pdf");
    pdfa_core::pagine::riordina(&tre, &rio, &[3, 2, 1]).unwrap();
    assert_eq!(n_pagine(&rio), 3);

    // Unisci due documenti (3 + 2) -> 5 pagine.
    let due = crea(2, "due");
    let uni = std::env::temp_dir().join("pdfa_pagine_uni.pdf");
    pdfa_core::pagine::unisci(&[tre.clone(), due], &uni).unwrap();
    assert_eq!(n_pagine(&uni), 5);

    // Ruota la pagina 1 di 90 gradi.
    let rot = std::env::temp_dir().join("pdfa_pagine_rot.pdf");
    pdfa_core::pagine::ruota(&tre, &rot, &[1], 90).unwrap();
    let d = Document::load(&rot).unwrap();
    let pid = *d.get_pages().get(&1).unwrap();
    let rotate = d.get_object(pid).unwrap().as_dict().unwrap().get(b"Rotate").unwrap().as_i64().unwrap();
    assert_eq!(rotate, 90);
}
