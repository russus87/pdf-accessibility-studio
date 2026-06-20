//! Test del generatore di PDF realmente taggato (overlay + marked content).
//! Solo lopdf, niente Pdfium.

use lopdf::{Dictionary, Document, Object};

use pdfa_core::taggatura::{genera_taggato, BloccoTag, OpzioniTag};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF di 2 pagine con un minimo di contenuto, senza struttura.
fn crea_non_taggato() -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let mut kids = Vec::new();
    for _ in 0..2 {
        // Un content stream banale (un rettangolo), per verificare che il nostro
        // overlay venga *aggiunto* senza perdere il contenuto esistente.
        let content_id = doc.add_object(lopdf::Stream::new(
            Dictionary::new(),
            b"q 0 0 100 100 re f Q\n".to_vec(),
        ));
        let pid = doc.new_object_id();
        let mut page = Dictionary::new();
        page.set("Type", nome("Page"));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
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

    let path = std::env::temp_dir().join("pdfa_tag_origine.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn pdf_taggato_con_marked_content_e_doclang() {
    let origine = crea_non_taggato();

    let prima = pdfa_core::analizza(&origine).unwrap();
    assert!(!prima.ha_struct_tree);

    let blocchi = vec![
        BloccoTag {
            ruolo: "H1".into(),
            testo: "Titolo àccentàto".into(),
            pagina: 0,
            bbox: Some([72.0, 700.0, 500.0, 720.0]),
            coord_basso: true,
            alt: None,
        },
        BloccoTag {
            ruolo: "P".into(),
            testo: "Un paragrafo di prova.".into(),
            pagina: 0,
            bbox: None,
            coord_basso: true,
            alt: None,
        },
        BloccoTag {
            ruolo: "Figure".into(),
            testo: String::new(),
            pagina: 1,
            bbox: Some([100.0, 100.0, 300.0, 300.0]),
            coord_basso: true,
            alt: Some("Un grafico a barre".into()),
        },
    ];

    let opt = OpzioniTag {
        lang: Some("it".into()),
        titolo: Some("Documento taggato".into()),
        doclang: Some("<doc>contenuto doclang</doc>".into()),
        pdfa3: true,
    };

    let dest = std::env::temp_dir().join("pdfa_tag_out.pdf");
    let n = genera_taggato(&origine, &dest, &blocchi, &opt).unwrap();
    assert_eq!(n, 3);

    // 1) Deve essere taggato e con struttura, secondo l'analizzatore del core.
    let dopo = pdfa_core::analizza(&dest).unwrap();
    assert!(dopo.taggato);
    assert!(dopo.ha_struct_tree);
    assert_eq!(dopo.lang.as_deref(), Some("it"));
    assert_eq!(dopo.titolo.as_deref(), Some("Documento taggato"));

    // 2) Ricarica grezza per i controlli di basso livello.
    let doc = Document::load(&dest).unwrap();

    // Il contenuto originale di pagina 0 deve essere preservato + il nostro
    // overlay aggiunto (Contents diventa un array con >= 2 stream).
    let (_n, page0) = doc.get_pages().into_iter().next().unwrap();
    let pdict = doc.get_object(page0).unwrap().as_dict().unwrap();
    match pdict.get(b"Contents").unwrap() {
        Object::Array(a) => assert!(a.len() >= 2, "overlay aggiunto senza perdere l'originale"),
        _ => panic!("Contents dovrebbe essere un array dopo l'overlay"),
    }
    // La pagina deve avere /StructParents e una risorsa font /F1.
    assert!(pdict.get(b"StructParents").is_ok());

    // 3) Marked content "BDC" presente in un content stream della pagina.
    let mut trovato_bdc = false;
    let mut trovato_tj = false;
    if let Ok(Object::Array(a)) = pdict.get(b"Contents") {
        for o in a {
            if let Object::Reference(r) = o {
                if let Ok(s) = doc.get_object(*r).and_then(|o| o.as_stream()) {
                    let testo = String::from_utf8_lossy(&s.content);
                    if testo.contains("BDC") {
                        trovato_bdc = true;
                    }
                    if testo.contains("Tj") {
                        trovato_tj = true;
                    }
                }
            }
        }
    }
    assert!(trovato_bdc, "deve esserci marked content (BDC) nell'overlay");
    assert!(trovato_tj, "deve esserci testo (Tj) nell'overlay");

    // 4) DocLang incorporato: il catalogo deve avere /AF e /Names/EmbeddedFiles.
    let catalog = {
        let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        doc.get_object(root).unwrap().as_dict().unwrap().clone()
    };
    assert!(catalog.get(b"AF").is_ok(), "deve esserci /AF (associated file)");
    assert!(catalog.get(b"Metadata").is_ok(), "PDF/A: deve esserci /Metadata XMP");
    assert!(catalog.get(b"OutputIntents").is_ok(), "PDF/A: deve esserci OutputIntent");
    // La figura deve avere l'Alt.
    let alt_figura = dopo
        .radice
        .iter()
        .flat_map(|nodo| nodo.figli.iter())
        .find(|nodo| nodo.ruolo == "Figure")
        .and_then(|nodo| nodo.alt.clone());
    assert_eq!(alt_figura.as_deref(), Some("Un grafico a barre"));
}
