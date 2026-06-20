//! Test dell'accessibilità moduli (Fase 20): lettura campi AcroForm e /TU.

use lopdf::{Dictionary, Document, Object};

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF con un AcroForm: un campo di testo "cognome" (widget sulla pagina), senza /TU.
fn crea_pdf() -> (std::path::PathBuf, String) {
    let mut doc = Document::with_version("1.7");

    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let field_id = doc.new_object_id();

    // Campo testo + widget unito (pratica comune): /FT Tx /T (cognome) /Subtype Widget.
    let mut field = Dictionary::new();
    field.set("FT", nome("Tx"));
    field.set("T", Object::string_literal("cognome"));
    field.set("Subtype", nome("Widget"));
    field.set("Rect", Object::Array(vec![100.into(), 600.into(), 300.into(), 620.into()]));
    field.set("P", Object::Reference(page_id));
    doc.set_object(field_id, field);

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
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

    let path = std::env::temp_dir().join("pdfa_test_form.pdf");
    doc.save(&path).unwrap();
    (path, format!("{}_{}", field_id.0, field_id.1))
}

#[test]
fn legge_campo_e_imposta_tooltip() {
    let (path, rif) = crea_pdf();

    // Lettura: un campo Tx "cognome", senza tooltip, sulla pagina 0.
    let campi = pdfa_core::form::leggi(&path).expect("leggi");
    assert_eq!(campi.len(), 1, "atteso un campo");
    let c = &campi[0];
    assert_eq!(c.nome.as_deref(), Some("cognome"));
    assert_eq!(c.tipo.as_deref(), Some("Tx"));
    assert_eq!(c.tooltip, None);
    assert_eq!(c.pagina, Some(0));
    assert_eq!(c.riferimento, rif);

    // Imposta il tooltip e attiva /Tabs strutturale.
    let out = std::env::temp_dir().join("pdfa_test_form_out.pdf");
    let mod_campi = vec![pdfa_core::form::ModificaCampo {
        riferimento: rif.clone(),
        tooltip: "Cognome".into(),
    }];
    let n = pdfa_core::form::applica(&path, &out, &mod_campi, true).expect("applica");
    assert_eq!(n, 1);

    // Rilettura: il tooltip ora c'è.
    let campi2 = pdfa_core::form::leggi(&out).expect("leggi dopo");
    assert_eq!(campi2[0].tooltip.as_deref(), Some("Cognome"));

    // /Tabs /S impostato sulla pagina.
    let doc = lopdf::Document::load(&out).unwrap();
    let page_id = doc.get_pages().into_values().next().unwrap();
    let tabs = doc.get_object(page_id).unwrap().as_dict().unwrap().get(b"Tabs").unwrap();
    assert_eq!(tabs.as_name().unwrap(), b"S");
}
