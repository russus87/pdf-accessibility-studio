//! Test delle evolutive (Fasi 27-30): cifratura, conversione, intestazioni, redazione.

use std::io::Cursor;
use std::sync::Mutex;

use lopdf::{Dictionary, Document, Object, Stream};

// Pdfium non va usato da più test in parallelo nello stesso processo: serializza.
static PDFIUM_LOCK: Mutex<()> = Mutex::new(());

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// PDF di una pagina A4: se `testo` è Some, include un content stream con quel testo.
fn crea_pdf(testo: Option<&str>) -> std::path::PathBuf {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();

    let mut page = Dictionary::new();
    page.set("Type", nome("Page"));
    page.set("Parent", Object::Reference(pages_id));
    page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]));

    if let Some(t) = testo {
        let mut font = Dictionary::new();
        font.set("Type", nome("Font"));
        font.set("Subtype", nome("Type1"));
        font.set("BaseFont", nome("Helvetica"));
        font.set("Encoding", nome("WinAnsiEncoding"));
        let font_id = doc.add_object(font);
        let contenuto = format!("BT /F1 24 Tf 100 700 Td ({t}) Tj ET");
        let content_id = doc.add_object(Stream::new(Dictionary::new(), contenuto.into_bytes()));
        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Reference(font_id));
        let mut res = Dictionary::new();
        res.set("Font", Object::Dictionary(fonts));
        page.set("Resources", Object::Dictionary(res));
        page.set("Contents", Object::Reference(content_id));
    } else {
        page.set("Resources", Object::Dictionary(Dictionary::new()));
        page.set("Contents", Object::Array(vec![]));
    }
    doc.set_object(page_id, page);

    let mut pages = Dictionary::new();
    pages.set("Type", nome("Pages"));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", 1);
    doc.set_object(pages_id, pages);

    let mut catalog = Dictionary::new();
    catalog.set("Type", nome("Catalog"));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let path = std::env::temp_dir().join(format!("pdfa_evo_{}.pdf", testo.unwrap_or("vuoto").replace(' ', "_")));
    doc.save(&path).unwrap();
    path
}

fn png_piccolo() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(12, 8, image::Rgb([180, 90, 40]));
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    buf.into_inner()
}

fn pdfium_ok() -> bool {
    pdfa_core::pdfium::inizializza(&[]).is_ok()
}

#[test]
fn cifratura_proteggi_e_rimuovi() {
    let input = crea_pdf(None);
    let prot = std::env::temp_dir().join("pdfa_evo_protetto.pdf");
    let aperto = std::env::temp_dir().join("pdfa_evo_aperto.pdf");

    pdfa_core::cifratura::proteggi(&input, &prot, "apri123", "owner456", &Default::default())
        .expect("proteggi");
    assert!(pdfa_core::cifratura::e_protetto(&prot).expect("check"), "il PDF deve essere protetto");

    pdfa_core::cifratura::rimuovi_protezione(&prot, &aperto, "apri123").expect("rimuovi");
    assert!(!pdfa_core::cifratura::e_protetto(&aperto).expect("check2"), "protezione rimossa");
}

#[test]
fn conversione_immagini_e_pdf() {
    let _g = PDFIUM_LOCK.lock().unwrap();
    if !pdfium_ok() {
        eprintln!("Pdfium non disponibile: salto");
        return;
    }
    let png = png_piccolo();
    let out = std::env::temp_dir().join("pdfa_evo_img2pdf.pdf");
    let n = pdfa_core::conversione::immagini_a_pdf(&[png.clone(), png], &out).expect("img->pdf");
    assert_eq!(n, 2);
    assert_eq!(pdfa_core::apri(&out).expect("apri").pagine, 2);

    let imgs = pdfa_core::conversione::pdf_a_immagini(&out, 200, false).expect("pdf->img");
    assert_eq!(imgs.len(), 2);
    assert_eq!(&imgs[0][0..4], &[137, 80, 78, 71], "intestazione PNG");
}

#[test]
fn intestazioni_numerazione() {
    let _g = PDFIUM_LOCK.lock().unwrap();
    if !pdfium_ok() {
        eprintln!("Pdfium non disponibile: salto");
        return;
    }
    let input = crea_pdf(None);
    let out = std::env::temp_dir().join("pdfa_evo_numerato.pdf");
    let op = pdfa_core::intestazioni::Opzioni {
        testo: "Pagina {n} di {tot}".into(),
        dim_pt: 12.0,
        colore: Default::default(),
        margine_mm: 12.0,
        ancora: pdfa_core::intestazioni::Ancora::BassoCentro,
        inizio_numerazione: 1,
    };
    let n = pdfa_core::intestazioni::applica(&input, &out, &op).expect("intestazioni");
    assert_eq!(n, 1);
    let testo = pdfa_core::testo_pagine(&out).expect("testo").join(" ");
    assert!(testo.contains("Pagina 1 di 1"), "numerazione presente: {testo:?}");
}

#[test]
fn redazione_rimuove_il_testo() {
    let _g = PDFIUM_LOCK.lock().unwrap();
    if !pdfium_ok() {
        eprintln!("Pdfium non disponibile: salto");
        return;
    }
    let input = crea_pdf(Some("SEGRETO"));
    let prima = pdfa_core::testo_pagine(&input).expect("prima").join(" ");
    assert!(prima.contains("SEGRETO"), "il testo c'è prima: {prima:?}");

    let out = std::env::temp_dir().join("pdfa_evo_redatto.pdf");
    let aree = vec![pdfa_core::redazione::Area {
        pagina: 0,
        x_mm: 0.0,
        y_mm: 0.0,
        larghezza_mm: 230.0,
        altezza_mm: 320.0,
    }];
    pdfa_core::redazione::redigi(&input, &out, &aree).expect("redigi");
    let dopo = pdfa_core::testo_pagine(&out).expect("dopo").join(" ");
    assert!(!dopo.contains("SEGRETO"), "il testo è stato rimosso: {dopo:?}");
}
