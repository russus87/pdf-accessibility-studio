//! Lettura e scrittura dei metadati del documento (dizionario Info + /Lang del
//! catalogo + ViewerPreferences/DisplayDocTitle), tramite lopdf.
//!
//! La scrittura salva su `destinazione`: il chiamante decide se e' una copia o
//! l'originale (sovrascrittura). L'originale non viene mai modificato in place.

use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Metadati letti dal documento (campi vuoti = assenti).
#[derive(Debug, Clone, Serialize, Default)]
pub struct Metadati {
    pub titolo: Option<String>,
    pub autore: Option<String>,
    pub soggetto: Option<String>,
    pub parole_chiave: Option<String>,
    pub creatore: Option<String>,
    pub produttore: Option<String>,
    /// Data di creazione (resa leggibile se in formato PDF "D:...").
    pub creato: Option<String>,
    /// Data di ultima modifica.
    pub modificato: Option<String>,
    /// Lingua a livello documento (/Lang nel catalogo).
    pub lang: Option<String>,
    /// ViewerPreferences/DisplayDocTitle (mostra il titolo nella barra del lettore).
    pub display_doc_title: Option<bool>,
    /// Il documento e' taggato (MarkInfo/Marked = true).
    pub taggato: bool,
}

/// Campi modificabili dall'utente (stringa vuota = rimuovi la chiave).
#[derive(Debug, Clone, Default)]
pub struct MetadatiInput {
    pub titolo: String,
    pub autore: String,
    pub soggetto: String,
    pub parole_chiave: String,
    pub creatore: String,
    pub produttore: String,
    pub lang: String,
    pub display_doc_title: bool,
}

/// Legge i metadati del documento.
pub fn leggi(percorso: &Path) -> Risultato<Metadati> {
    let doc = Document::load(percorso).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let info = deref(&doc, doc.trailer.get(b"Info").ok()).and_then(|o| o.as_dict().ok());
    let g = |chiave: &[u8]| info.and_then(|d| stringa(&doc, d, chiave)).filter(|s| !s.trim().is_empty());

    let catalogo = deref(&doc, doc.trailer.get(b"Root").ok()).and_then(|o| o.as_dict().ok());

    let lang = catalogo
        .and_then(|c| stringa(&doc, c, b"Lang"))
        .filter(|s| !s.trim().is_empty());

    let display_doc_title = catalogo.and_then(|c| {
        deref(&doc, c.get(b"ViewerPreferences").ok())
            .and_then(|o| o.as_dict().ok())
            .and_then(|vp| vp.get(b"DisplayDocTitle").ok())
            .and_then(|o| o.as_bool().ok())
    });

    let taggato = catalogo
        .and_then(|c| deref(&doc, c.get(b"MarkInfo").ok()))
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Marked").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);

    Ok(Metadati {
        titolo: g(b"Title"),
        autore: g(b"Author"),
        soggetto: g(b"Subject"),
        parole_chiave: g(b"Keywords"),
        creatore: g(b"Creator"),
        produttore: g(b"Producer"),
        creato: g(b"CreationDate").map(|d| data_leggibile(&d)),
        modificato: g(b"ModDate").map(|d| data_leggibile(&d)),
        lang,
        display_doc_title,
        taggato,
    })
}

/// Applica i metadati e salva in `destinazione` (copia o sovrascrittura).
pub fn scrivi(origine: &Path, destinazione: &Path, m: &MetadatiInput) -> Risultato<()> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo (Root) non trovato".into()))?;

    // --- Catalogo: Lang + ViewerPreferences/DisplayDocTitle ---
    let mut vp = leggi_viewer_preferences(&doc, catalog_id).unwrap_or_default();
    {
        let catalog = doc
            .get_object_mut(catalog_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;
        if m.lang.trim().is_empty() {
            catalog.remove(b"Lang");
        } else {
            catalog.set("Lang", Object::string_literal(m.lang.trim()));
        }
        vp.set("DisplayDocTitle", Object::Boolean(m.display_doc_title));
        catalog.set("ViewerPreferences", Object::Dictionary(vp));
    }

    // --- Info ---
    let info_id = match doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let id = doc.add_object(Dictionary::new());
            doc.trailer.set("Info", Object::Reference(id));
            id
        }
    };
    let info = doc
        .get_object_mut(info_id)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| Errore::Pdfium(format!("Info: {e}")))?;

    imposta(info, "Title", &m.titolo);
    imposta(info, "Author", &m.autore);
    imposta(info, "Subject", &m.soggetto);
    imposta(info, "Keywords", &m.parole_chiave);
    imposta(info, "Creator", &m.creatore);
    imposta(info, "Producer", &m.produttore);

    doc.save(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// Imposta una chiave testuale dell'Info, o la rimuove se il valore e' vuoto.
fn imposta(info: &mut Dictionary, chiave: &str, valore: &str) {
    if valore.trim().is_empty() {
        info.remove(chiave.as_bytes());
    } else {
        info.set(chiave, stringa_utf16(valore));
    }
}

/// Restituisce una copia del dizionario ViewerPreferences, se presente.
fn leggi_viewer_preferences(doc: &Document, catalog_id: ObjectId) -> Option<Dictionary> {
    let catalog = doc.get_object(catalog_id).ok()?.as_dict().ok()?;
    match catalog.get(b"ViewerPreferences").ok()? {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok().cloned(),
        Object::Dictionary(d) => Some(d.clone()),
        _ => None,
    }
}

/// Converte una data PDF "D:YYYYMMDDHHmmSS..." in "YYYY-MM-DD HH:mm" (best-effort).
fn data_leggibile(d: &str) -> String {
    let s = d.trim().trim_start_matches("D:");
    let n: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if n.len() < 8 {
        return d.to_string();
    }
    let (a, me, g) = (&n[0..4], &n[4..6], &n[6..8]);
    if n.len() >= 12 {
        format!("{a}-{me}-{g} {}:{}", &n[8..10], &n[10..12])
    } else {
        format!("{a}-{me}-{g}")
    }
}

// --- Helper (analoghi a struttura.rs) ----------------------------------------

fn deref<'a>(doc: &'a Document, obj: Option<&'a Object>) -> Option<&'a Object> {
    let mut cur = obj?;
    for _ in 0..32 {
        match cur.as_reference() {
            Ok(id) => match doc.get_object(id) {
                Ok(o) => cur = o,
                Err(_) => return None,
            },
            Err(_) => return Some(cur),
        }
    }
    Some(cur)
}

fn stringa(doc: &Document, d: &Dictionary, chiave: &[u8]) -> Option<String> {
    match deref(doc, d.get(chiave).ok())? {
        Object::String(b, _) => Some(decodifica(b)),
        Object::Name(b) => Some(String::from_utf8_lossy(b).into_owned()),
        _ => None,
    }
}

fn decodifica(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16s: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|c| ((c[0] as u16) << 8) | (*c.get(1).unwrap_or(&0) as u16))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
