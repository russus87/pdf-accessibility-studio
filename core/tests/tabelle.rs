//! Test dell'editor di accessibilità tabelle (Fase 19): Scope su TH e ColSpan.

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF taggato con una tabella: Table > TR > [TH, TD]. Nessun attributo Scope.
fn crea_pdf() -> (std::path::PathBuf, String, String) {
    let mut doc = Document::with_version("1.7");

    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let struct_root_id = doc.new_object_id();

    let elem = |doc: &mut Document, s: &str, parent: lopdf::ObjectId, k: Option<Object>| {
        let mut d = Dictionary::new();
        d.set("Type", nome("StructElem"));
        d.set("S", nome(s));
        d.set("Pg", Object::Reference(page_id));
        d.set("P", Object::Reference(parent));
        if let Some(k) = k {
            d.set("K", k);
        }
        doc.add_object(d)
    };

    let table_id = doc.new_object_id();
    let tr_id = doc.new_object_id();
    let th_id = elem(&mut doc, "TH", tr_id, None);
    let td_id = elem(&mut doc, "TD", tr_id, None);
    // TR e Table (creati dopo per avere gli id dei figli).
    let mut tr = Dictionary::new();
    tr.set("Type", nome("StructElem"));
    tr.set("S", nome("TR"));
    tr.set("P", Object::Reference(table_id));
    tr.set("K", Object::Array(vec![Object::Reference(th_id), Object::Reference(td_id)]));
    doc.set_object(tr_id, tr);

    let mut table = Dictionary::new();
    table.set("Type", nome("StructElem"));
    table.set("S", nome("Table"));
    table.set("P", Object::Reference(struct_root_id));
    table.set("K", Object::Reference(tr_id));
    doc.set_object(table_id, table);

    let mut root = Dictionary::new();
    root.set("Type", nome("StructTreeRoot"));
    root.set("K", Object::Reference(table_id));
    doc.set_object(struct_root_id, root);

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
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
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join("pdfa_test_tabella.pdf");
    doc.save(&path).unwrap();
    (
        path,
        format!("{}_{}", th_id.0, th_id.1),
        format!("{}_{}", td_id.0, td_id.1),
    )
}

#[test]
fn scope_e_colspan() {
    let (path, rif_th, rif_td) = crea_pdf();

    // Stato iniziale: nessuno scope sul TH.
    let info = pdfa_core::analizza(&path).expect("analizza");
    let th = trova(&info.radice, &rif_th).expect("TH presente");
    assert_eq!(th.scope, None);

    // Applica Scope=Column al TH e ColSpan=2 al TD.
    let out = std::env::temp_dir().join("pdfa_test_tabella_out.pdf");
    let celle = vec![
        pdfa_core::tabelle::AttributiCella {
            riferimento: rif_th.clone(),
            scope: Some("Column".into()),
            ..Default::default()
        },
        pdfa_core::tabelle::AttributiCella {
            riferimento: rif_td.clone(),
            col_span: Some(2),
            ..Default::default()
        },
    ];
    let n = pdfa_core::tabelle::applica(&path, &out, &celle).expect("applica");
    assert_eq!(n, 2);

    // Dopo: lo scope e lo span risultano nella struttura riletta.
    let info2 = pdfa_core::analizza(&out).expect("analizza dopo");
    let th2 = trova(&info2.radice, &rif_th).expect("TH dopo");
    assert_eq!(th2.scope.as_deref(), Some("Column"));
    let td2 = trova(&info2.radice, &rif_td).expect("TD dopo");
    assert_eq!(td2.col_span, Some(2));
}

fn trova<'a>(nodi: &'a [pdfa_core::struttura::NodoTag], rif: &str) -> Option<&'a pdfa_core::struttura::NodoTag> {
    for n in nodi {
        if n.riferimento.as_deref() == Some(rif) {
            return Some(n);
        }
        if let Some(t) = trova(&n.figli, rif) {
            return Some(t);
        }
    }
    None
}
