//! Esportazione verso Office (Fase 40): PDF → Word (.docx) e PDF → Excel (.xlsx).
//!
//! I file OOXML sono archivi ZIP con parti XML. Il Word usa l'ordine logico dei
//! tag (titoli/paragrafi); l'Excel estrae le tabelle (una sotto l'altra).

use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;

use crate::errore::{Errore, Risultato};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn livello(ruolo: &str) -> Option<u8> {
    match ruolo {
        "Title" | "H1" => Some(1),
        _ => ruolo.strip_prefix('H').and_then(|s| s.parse::<u8>().ok()).filter(|n| (1..=6).contains(n)),
    }
}

/// Scrive un archivio ZIP con le parti (nome, contenuto) date.
fn scrivi_zip(dest: &Path, parti: &[(&str, String)]) -> Risultato<()> {
    let file = std::fs::File::create(dest).map_err(|e| Errore::Io(format!("creazione: {e}")))?;
    let mut z = zip::ZipWriter::new(file);
    let opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nome, contenuto) in parti {
        z.start_file(*nome, opt).map_err(|e| Errore::Io(format!("zip: {e}")))?;
        z.write_all(contenuto.as_bytes()).map_err(|e| Errore::Io(format!("zip write: {e}")))?;
    }
    z.finish().map_err(|e| Errore::Io(format!("zip finish: {e}")))?;
    Ok(())
}

/// Esporta in Word (.docx) usando titoli e paragrafi dall'ordine logico dei tag.
pub fn pdf_a_docx(origine: &Path, destinazione: &Path) -> Risultato<()> {
    let blocchi = crate::lettura::blocchi(origine).unwrap_or_default();
    let righe: Vec<(String, String)> = if blocchi.is_empty() {
        crate::testo_pagine(origine)?.into_iter().map(|t| (String::new(), t)).collect()
    } else {
        blocchi.iter().map(|b| {
            let stile = match livello(&b.ruolo) {
                Some(n) => format!("Heading{n}"),
                None => String::new(),
            };
            (stile, b.testo.clone())
        }).collect()
    };

    let mut body = String::new();
    for (stile, testo) in &righe {
        if testo.trim().is_empty() { continue; }
        let ppr = if stile.is_empty() { String::new() } else { format!("<w:pPr><w:pStyle w:val=\"{stile}\"/></w:pPr>") };
        body.push_str(&format!("<w:p>{ppr}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>", esc(testo)));
    }

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>"
    );

    scrivi_zip(destinazione, &[
        ("[Content_Types].xml", content_types.to_string()),
        ("_rels/.rels", rels.to_string()),
        ("word/document.xml", document),
    ])
}

/// Lettera di colonna Excel da indice 0-based (0→A, 26→AA).
fn colonna(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 { break; }
        n = n / 26 - 1;
    }
    s
}

/// Esporta in Excel (.xlsx) le tabelle del documento (una sotto l'altra). Se non
/// ci sono tabelle, esporta il testo (una riga per blocco) nella colonna A.
pub fn pdf_a_xlsx(origine: &Path, destinazione: &Path) -> Risultato<()> {
    let tabelle = crate::lettura::tabelle(origine).unwrap_or_default();

    let mut righe_xml = String::new();
    let mut r = 1usize;
    let mut aggiungi_riga = |celle: &[String], r: &mut usize| {
        let mut cs = String::new();
        for (i, c) in celle.iter().enumerate() {
            cs.push_str(&format!("<c r=\"{}{}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", colonna(i), r, esc(c)));
        }
        righe_xml.push_str(&format!("<row r=\"{}\">{}</row>", r, cs));
        *r += 1;
    };

    if tabelle.is_empty() {
        let testo = crate::testo_pagine(origine)?;
        for t in testo.iter().flat_map(|p| p.lines()) {
            if !t.trim().is_empty() { aggiungi_riga(&[t.to_string()], &mut r); }
        }
    } else {
        for (n, tab) in tabelle.iter().enumerate() {
            if n > 0 { r += 1; } // riga vuota tra tabelle
            for riga in tab {
                aggiungi_riga(riga, &mut r);
            }
        }
    }

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
    let workbook = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Foglio1" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
    let wb_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#;
    let sheet = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>{righe_xml}</sheetData></worksheet>"
    );

    scrivi_zip(destinazione, &[
        ("[Content_Types].xml", content_types.to_string()),
        ("_rels/.rels", rels.to_string()),
        ("xl/workbook.xml", workbook.to_string()),
        ("xl/_rels/workbook.xml.rels", wb_rels.to_string()),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}
