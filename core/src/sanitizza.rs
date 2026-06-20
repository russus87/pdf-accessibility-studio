//! Sanitizzazione/privacy (Fase 44): rimuove dati nascosti dal PDF —
//! JavaScript e azioni automatiche, metadati (Info + XMP), file allegati.

use std::path::Path;

use lopdf::{Dictionary, Document, Object};
use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Cosa rimuovere.
#[derive(Debug, Clone, Copy)]
pub struct Opzioni {
    pub javascript: bool,
    pub metadati: bool,
    pub allegati: bool,
}

impl Default for Opzioni {
    fn default() -> Self {
        Opzioni { javascript: true, metadati: true, allegati: true }
    }
}

/// Cosa è stato effettivamente rimosso.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Esito {
    pub javascript: bool,
    pub metadati: bool,
    pub allegati: bool,
}

/// Rimuove i dati nascosti richiesti e salva una copia.
pub fn pulisci(origine: &Path, destinazione: &Path, op: &Opzioni) -> Risultato<Esito> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;
    let mut esito = Esito::default();

    let catalog_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo non trovato".into()))?;

    // Helper: rimuove una voce dal sotto-dizionario /Names del catalogo.
    let names_id = doc.get_object(catalog_id).ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|c| c.get(b"Names").ok())
        .and_then(|o| o.as_reference().ok());

    if op.javascript {
        if let Ok(cat) = doc.get_object_mut(catalog_id).and_then(|o| o.as_dict_mut()) {
            for k in [b"OpenAction".as_slice(), b"AA".as_slice()] {
                if cat.remove(k).is_some() { esito.javascript = true; }
            }
        }
        if let Some(nid) = names_id {
            if let Ok(names) = doc.get_object_mut(nid).and_then(|o| o.as_dict_mut()) {
                if names.remove(b"JavaScript").is_some() { esito.javascript = true; }
            }
        }
    }

    if op.allegati {
        if let Some(nid) = names_id {
            if let Ok(names) = doc.get_object_mut(nid).and_then(|o| o.as_dict_mut()) {
                if names.remove(b"EmbeddedFiles").is_some() { esito.allegati = true; }
            }
        }
    }

    if op.metadati {
        // /Metadata (XMP) dal catalogo.
        if let Ok(cat) = doc.get_object_mut(catalog_id).and_then(|o| o.as_dict_mut()) {
            if cat.remove(b"Metadata").is_some() { esito.metadati = true; }
        }
        // Dizionario Info dal trailer.
        if let Ok(info_id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
            if let Ok(info) = doc.get_object_mut(info_id).and_then(|o| o.as_dict_mut()) {
                let chiavi: Vec<Vec<u8>> = info.iter().map(|(k, _)| k.to_vec()).collect();
                for k in chiavi {
                    // Mantiene eventuale Title (utile all'accessibilità); rimuove il resto.
                    if k != b"Title" { info.remove(&k); }
                }
                esito.metadati = true;
            }
        }
    }

    // Rimuove eventuali azioni JavaScript dalle annotazioni.
    if op.javascript {
        let ids: Vec<_> = doc.objects.keys().copied().collect();
        for id in ids {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                if e_annotazione_con_js(d) {
                    d.remove(b"A");
                    d.remove(b"AA");
                    esito.javascript = true;
                }
            }
        }
    }

    doc.prune_objects();
    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(esito)
}

/// Vero se il dizionario è un'annotazione/azione con /A o /AA di tipo JavaScript.
fn e_annotazione_con_js(d: &Dictionary) -> bool {
    let js = |o: Option<&Object>| {
        matches!(o, Some(Object::Dictionary(a)) if a.get(b"S").and_then(|s| s.as_name()).map(|n| n == b"JavaScript").unwrap_or(false))
    };
    js(d.get(b"A").ok()) || d.get(b"AA").is_ok() && matches!(d.get(b"AA"), Ok(Object::Dictionary(_)))
}
