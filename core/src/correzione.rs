//! Correzione assistita: applica correzioni di accessibilita' a livello
//! documento e salva una copia corretta (senza toccare l'originale).
//!
//! Per ora: lingua (/Lang), titolo (Info/Title) e DisplayDocTitle. Questi
//! risolvono diversi esiti della validazione. L'aggiunta di Alt alle singole
//! figure richiede un'interfaccia dedicata e arrivera' in seguito.

use std::path::Path;

use lopdf::{Dictionary, Document, Object, StringFormat};

use crate::errore::{Errore, Risultato};

/// Correzioni richieste dall'utente.
#[derive(Debug, Default)]
pub struct Correzioni {
    pub lang: Option<String>,
    pub titolo: Option<String>,
    pub display_doc_title: bool,
}

/// Applica le correzioni a `origine` e salva il risultato in `destinazione`.
pub fn applica(origine: &Path, destinazione: &Path, c: &Correzioni) -> Risultato<()> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo (Root) non trovato".into()))?;

    // Legge le ViewerPreferences esistenti (per non perderne altre chiavi).
    let mut vp = leggi_viewer_preferences(&doc, catalog_id).unwrap_or_default();

    // --- Catalogo: Lang + ViewerPreferences/DisplayDocTitle ---
    {
        let catalog = doc
            .get_object_mut(catalog_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;

        if let Some(l) = &c.lang {
            catalog.set("Lang", Object::string_literal(l.as_str()));
        }
        if c.display_doc_title {
            vp.set("DisplayDocTitle", Object::Boolean(true));
            catalog.set("ViewerPreferences", Object::Dictionary(vp));
        }
    }

    // --- Info/Title ---
    if let Some(t) = &c.titolo {
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
        info.set("Title", stringa_utf16(t));
    }

    doc.save(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// Restituisce una copia del dizionario ViewerPreferences, se presente.
fn leggi_viewer_preferences(doc: &Document, catalog_id: lopdf::ObjectId) -> Option<Dictionary> {
    let catalog = doc.get_object(catalog_id).ok()?.as_dict().ok()?;
    match catalog.get(b"ViewerPreferences").ok()? {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok().cloned(),
        Object::Dictionary(d) => Some(d.clone()),
        _ => None,
    }
}

/// Codifica una stringa come testo PDF UTF-16BE (gestisce gli accenti).
fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
