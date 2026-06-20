//! Accessibilità dei moduli (AcroForm) — Fase 20.
//!
//! I campi modulo sono accessibili se hanno un'etichetta che lo screen reader può
//! annunciare: in PDF è il **tooltip** `/TU` (alternate field name). Questo modulo
//! legge i campi dell'AcroForm (tipo, nome, tooltip, pagina) e permette di
//! scrivere il `/TU` mancante, oltre a impostare l'ordine di tabulazione
//! strutturale (`/Tabs /S`) sulle pagine. Salva sempre una copia.

use std::collections::HashMap;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Un campo modulo letto dall'AcroForm.
#[derive(Debug, Clone, Serialize)]
pub struct CampoModulo {
    /// Riferimento "numero_generazione" del campo.
    pub riferimento: String,
    /// Nome interno del campo (/T).
    pub nome: Option<String>,
    /// Tipo del campo (/FT): "Tx" testo, "Btn" pulsante/checkbox, "Ch" scelta, "Sig" firma.
    pub tipo: Option<String>,
    /// Tooltip / etichetta accessibile (/TU), se presente.
    pub tooltip: Option<String>,
    /// Pagina (0-based) su cui appare il widget, se ricavabile.
    pub pagina: Option<i32>,
}

/// Modifica richiesta per un campo: imposta il tooltip /TU.
#[derive(Debug, Clone)]
pub struct ModificaCampo {
    pub riferimento: String,
    pub tooltip: String,
}

/// Legge i campi dell'AcroForm. Vuoto se il documento non ha un modulo.
pub fn leggi(percorso: &Path) -> Risultato<Vec<CampoModulo>> {
    let doc = Document::load(percorso).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let Some(catalog) = catalogo(&doc) else {
        return Ok(Vec::new());
    };
    let Some(acro) = deref(&doc, catalog.get(b"AcroForm").ok()).and_then(|o| o.as_dict().ok()) else {
        return Ok(Vec::new());
    };
    let Some(fields) = deref(&doc, acro.get(b"Fields").ok()).and_then(|o| o.as_array().ok()) else {
        return Ok(Vec::new());
    };

    // Mappa widget(annotazione) -> indice pagina, scorrendo gli /Annots.
    let mut annot_pagina: HashMap<ObjectId, i32> = HashMap::new();
    for (num, page_id) in doc.get_pages() {
        if let Ok(page) = doc.get_object(page_id).and_then(|o| o.as_dict()) {
            if let Some(Object::Array(annots)) = deref(&doc, page.get(b"Annots").ok()) {
                for a in annots {
                    if let Ok(id) = a.as_reference() {
                        annot_pagina.insert(id, num as i32 - 1);
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    let mut visti = std::collections::HashSet::new();
    for f in fields {
        if let Ok(id) = f.as_reference() {
            raccogli_campo(&doc, id, None, &annot_pagina, &mut out, &mut visti, 0);
        }
    }
    Ok(out)
}

/// Raccoglie un campo (e i figli con nome proprio) nell'elenco.
fn raccogli_campo(
    doc: &Document,
    id: ObjectId,
    tipo_ered: Option<String>,
    annot_pagina: &HashMap<ObjectId, i32>,
    out: &mut Vec<CampoModulo>,
    visti: &mut std::collections::HashSet<ObjectId>,
    prof: usize,
) {
    if prof > 32 || !visti.insert(id) {
        return;
    }
    let Ok(d) = doc.get_object(id).and_then(|o| o.as_dict()) else {
        return;
    };

    let tipo = d
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .or(tipo_ered.clone());

    // I figli con /T proprio sono campi a sé; i widget puri (senza /T) appartengono
    // a questo campo e ne determinano la pagina.
    let kids: Vec<ObjectId> = deref(doc, d.get(b"Kids").ok())
        .and_then(|o| o.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default();

    let kids_campo: Vec<ObjectId> = kids
        .iter()
        .copied()
        .filter(|kid| {
            doc.get_object(*kid)
                .and_then(|o| o.as_dict())
                .map(|kd| kd.get(b"T").is_ok())
                .unwrap_or(false)
        })
        .collect();

    if kids_campo.is_empty() {
        // È un campo "foglia": pagina dal suo widget o dai widget figli.
        let pagina = annot_pagina.get(&id).copied().or_else(|| {
            kids.iter().find_map(|kid| annot_pagina.get(kid).copied())
        });
        out.push(CampoModulo {
            riferimento: format!("{}_{}", id.0, id.1),
            nome: testo(doc, d, b"T"),
            tipo,
            tooltip: testo(doc, d, b"TU"),
            pagina,
        });
    } else {
        for kid in kids_campo {
            raccogli_campo(doc, kid, tipo.clone(), annot_pagina, out, visti, prof + 1);
        }
    }
}

/// Applica le modifiche ai campi (imposta /TU) e, se richiesto, l'ordine di
/// tabulazione strutturale (/Tabs /S) su tutte le pagine. Salva una copia.
/// Ritorna quanti tooltip sono stati impostati.
pub fn applica(
    origine: &Path,
    destinazione: &Path,
    campi: &[ModificaCampo],
    tabs_struttura: bool,
) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let mut count = 0;
    for c in campi {
        if c.tooltip.trim().is_empty() {
            continue;
        }
        if let Some(id) = parse_riferimento(&c.riferimento) {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                d.set("TU", stringa_utf16(&c.tooltip));
                count += 1;
            }
        }
    }

    if tabs_struttura {
        let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
        for id in page_ids {
            if let Ok(page) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                page.set("Tabs", Object::Name(b"S".to_vec()));
            }
        }
    }

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(count)
}

// --- Helper ------------------------------------------------------------------

fn catalogo(doc: &Document) -> Option<&Dictionary> {
    deref(doc, doc.trailer.get(b"Root").ok())?.as_dict().ok()
}

fn deref<'a>(doc: &'a Document, obj: Option<&'a Object>) -> Option<&'a Object> {
    let mut cur = obj?;
    for _ in 0..32 {
        match cur.as_reference() {
            Ok(id) => cur = doc.get_object(id).ok()?,
            Err(_) => return Some(cur),
        }
    }
    Some(cur)
}

/// Legge una stringa testuale (gestisce UTF-16BE con BOM).
fn testo(doc: &Document, d: &Dictionary, chiave: &[u8]) -> Option<String> {
    let Object::String(b, _) = deref(doc, d.get(chiave).ok())? else {
        return None;
    };
    let s = if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
        let u: Vec<u16> = b[2..]
            .chunks(2)
            .map(|c| ((c[0] as u16) << 8) | (*c.get(1).unwrap_or(&0) as u16))
            .collect();
        String::from_utf16_lossy(&u)
    } else {
        b.iter().map(|&x| x as char).collect()
    };
    Some(s).filter(|s| !s.trim().is_empty())
}

fn parse_riferimento(s: &str) -> Option<ObjectId> {
    let (n, g) = s.split_once('_')?;
    Some((n.parse().ok()?, g.parse().ok()?))
}

/// Codifica una stringa come testo PDF UTF-16BE (gestisce gli accenti).
fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
