//! Estrazione del testo nell'ordine logico dei tag (ordine MCID).
//!
//! Diversamente dall'estrazione per pagina, qui percorriamo lo structure tree e,
//! per ogni elemento, raccogliamo il testo dei suoi marked-content (MCID),
//! decodificato con le stesse mappe font di lopdf. Il risultato e' la sequenza
//! di lettura "come uno screen reader", con il ruolo di ogni blocco.

use std::collections::HashMap;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Un blocco nella sequenza di lettura logica.
#[derive(Debug, Clone, Serialize)]
pub struct BloccoLettura {
    pub testo: String,
    pub ruolo: String,
    pub pagina: Option<i32>,
}

/// Restituisce i blocchi di lettura in ordine logico (tag/MCID). Vuoto se il
/// documento non e' taggato o non si ricava testo: il chiamante puo' allora
/// ripiegare sull'estrazione per pagina.
pub fn blocchi(percorso: &std::path::Path) -> Risultato<Vec<BloccoLettura>> {
    let doc = Document::load(percorso).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    // Indice di pagina (0-based) per ogni ObjectId di pagina.
    let mut idx_pagina: HashMap<ObjectId, i32> = HashMap::new();
    for (num, id) in doc.get_pages() {
        idx_pagina.insert(id, num as i32 - 1);
    }

    // Testo per (indice_pagina, MCID).
    let mut testi: HashMap<(i32, i32), String> = HashMap::new();
    for (num, id) in doc.get_pages() {
        estrai_pagina(&doc, id, num as i32 - 1, &mut testi);
    }

    // Radice dello structure tree.
    let Some(catalog) = deref(&doc, doc.trailer.get(b"Root").ok()).and_then(|o| o.as_dict().ok()) else {
        return Ok(Vec::new());
    };
    let Some(str_root) = deref(&doc, catalog.get(b"StructTreeRoot").ok()).and_then(|o| o.as_dict().ok())
    else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    if let Ok(k) = str_root.get(b"K") {
        let mut visti = std::collections::HashSet::new();
        elemento(&doc, k, None, &testi, &idx_pagina, &mut out, &mut visti, 0);
    }
    Ok(out)
}

/// Percorre un elemento (o lista di elementi) producendo i blocchi in ordine.
#[allow(clippy::too_many_arguments)]
fn elemento(
    doc: &Document,
    obj: &Object,
    pagina_ered: Option<i32>,
    testi: &HashMap<(i32, i32), String>,
    idx_pagina: &HashMap<ObjectId, i32>,
    out: &mut Vec<BloccoLettura>,
    visti: &mut std::collections::HashSet<ObjectId>,
    prof: usize,
) {
    if prof > 80 {
        return;
    }
    if let Ok(id) = obj.as_reference() {
        if !visti.insert(id) {
            return;
        }
    }
    let Some(ris) = deref(doc, Some(obj)) else { return };

    match ris {
        Object::Array(arr) => {
            for o in arr {
                elemento(doc, o, pagina_ered, testi, idx_pagina, out, visti, prof + 1);
            }
        }
        Object::Dictionary(d) => {
            // Solo gli elementi con S sono elementi strutturali.
            let Ok(s) = d.get(b"S").and_then(|o| o.as_name()) else {
                // Contenitore senza ruolo: scendi nei figli.
                if let Ok(k) = d.get(b"K") {
                    elemento(doc, k, pagina_ered, testi, idx_pagina, out, visti, prof + 1);
                }
                return;
            };
            let ruolo = String::from_utf8_lossy(s).into_owned();
            let pagina = d
                .get(b"Pg")
                .ok()
                .and_then(|o| o.as_reference().ok())
                .and_then(|id| idx_pagina.get(&id).copied())
                .or(pagina_ered);

            // Scorre K in ordine: il testo diretto (MCID) viene emesso prima di
            // scendere nei figli-elemento, cosi' l'ordine di lettura e' rispettato.
            let mut buffer = String::new();
            if let Ok(k) = d.get(b"K") {
                scorri_k(doc, k, &ruolo, pagina, testi, idx_pagina, &mut buffer, out, visti, prof);
            }
            spinta(out, &ruolo, pagina, &mut buffer);

            // Figura con testo alternativo: leggilo come fa uno screen reader.
            if ruolo == "Figure" {
                if let Some(alt) = stringa(doc, d, b"Alt") {
                    if !alt.trim().is_empty() {
                        out.push(BloccoLettura { testo: alt, ruolo: "Figure".into(), pagina });
                    }
                }
            }
        }
        _ => {}
    }
}

/// Scorre i figli K di un elemento, accumulando il testo diretto e ricorrendo
/// nei figli-elemento (svuotando prima il buffer per mantenere l'ordine).
#[allow(clippy::too_many_arguments)]
fn scorri_k(
    doc: &Document,
    k: &Object,
    ruolo: &str,
    pagina: Option<i32>,
    testi: &HashMap<(i32, i32), String>,
    idx_pagina: &HashMap<ObjectId, i32>,
    buffer: &mut String,
    out: &mut Vec<BloccoLettura>,
    visti: &mut std::collections::HashSet<ObjectId>,
    prof: usize,
) {
    let Some(ris) = deref(doc, Some(k)) else { return };
    match ris {
        Object::Array(arr) => {
            for o in arr {
                scorri_k(doc, o, ruolo, pagina, testi, idx_pagina, buffer, out, visti, prof);
            }
        }
        // MCID diretto: testo di questo elemento sulla sua pagina.
        Object::Integer(mcid) => {
            if let (Some(p), t) = (pagina, testi.get(&(pagina.unwrap_or(-1), *mcid as i32))) {
                let _ = p;
                if let Some(t) = t {
                    buffer.push_str(t);
                }
            }
        }
        Object::Dictionary(d) => {
            // MCR (riferimento a marked content) oppure elemento figlio.
            if d.get(b"S").is_ok() {
                // E' un elemento figlio: svuota il buffer e ricorri.
                spinta(out, ruolo, pagina, buffer);
                elemento(doc, ris, pagina, testi, idx_pagina, out, visti, prof + 1);
            } else if let Ok(mcid) = d.get(b"MCID").and_then(|o| o.as_i64()) {
                let p = d
                    .get(b"Pg")
                    .ok()
                    .and_then(|o| o.as_reference().ok())
                    .and_then(|id| idx_pagina.get(&id).copied())
                    .or(pagina);
                if let Some(t) = testi.get(&(p.unwrap_or(-1), mcid as i32)) {
                    buffer.push_str(t);
                }
            }
        }
        _ => {}
    }
}

/// Emette un blocco se il buffer ha testo significativo, poi lo svuota.
fn spinta(out: &mut Vec<BloccoLettura>, ruolo: &str, pagina: Option<i32>, buffer: &mut String) {
    let t = buffer.trim();
    if !t.is_empty() {
        out.push(BloccoLettura { testo: t.to_string(), ruolo: ruolo.to_string(), pagina });
    }
    buffer.clear();
}

/// Costruisce la mappa MCID -> testo per una pagina, decodificando con i font.
fn estrai_pagina(doc: &Document, page_id: ObjectId, idx: i32, testi: &mut HashMap<(i32, i32), String>) {
    let Ok(fonts) = doc.get_page_fonts(page_id) else { return };
    let encodings: HashMap<Vec<u8>, lopdf::Encoding> = fonts
        .into_iter()
        .filter_map(|(nome, font)| font.get_font_encoding(doc).ok().map(|e| (nome, e)))
        .collect();

    let Ok(dati) = doc.get_page_content(page_id) else { return };
    let Ok(content) = Content::decode(&dati) else { return };

    let mut enc: Option<&lopdf::Encoding> = None;
    let mut stack: Vec<i32> = Vec::new(); // MCID annidati (-1 = senza MCID)

    for op in &content.operations {
        match op.operator.as_ref() {
            "Tf" => {
                enc = op.operands.first().and_then(|o| o.as_name().ok()).and_then(|n| encodings.get(n));
            }
            "BDC" | "BMC" => stack.push(mcid_da_operandi(&op.operands).unwrap_or(-1)),
            "EMC" => {
                stack.pop();
            }
            "Tj" | "TJ" | "'" | "\"" => {
                if let Some(encoding) = enc {
                    let mut s = String::new();
                    raccogli(&mut s, encoding, op.operator.as_ref(), &op.operands);
                    if !s.is_empty() {
                        if let Some(&m) = stack.iter().rev().find(|&&m| m >= 0) {
                            testi.entry((idx, m)).or_default().push_str(&s);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Decodifica gli operandi di testo di un operatore in una stringa.
fn raccogli(out: &mut String, enc: &lopdf::Encoding, operatore: &str, operandi: &[Object]) {
    let decodifica = |out: &mut String, b: &[u8]| {
        if let Ok(t) = Document::decode_text(enc, b) {
            out.push_str(&t);
        }
    };
    match operatore {
        "Tj" | "'" => {
            if let Some(Object::String(b, _)) = operandi.last() {
                decodifica(out, b);
            }
        }
        "\"" => {
            if let Some(Object::String(b, _)) = operandi.get(2) {
                decodifica(out, b);
            }
        }
        "TJ" => {
            if let Some(Object::Array(arr)) = operandi.first() {
                for el in arr {
                    if let Object::String(b, _) = el {
                        decodifica(out, b);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Estrae il MCID dagli operandi di un BDC (proprieta' inline con /MCID).
fn mcid_da_operandi(operandi: &[Object]) -> Option<i32> {
    if let Some(Object::Dictionary(d)) = operandi.get(1) {
        return d.get(b"MCID").ok()?.as_i64().ok().map(|v| v as i32);
    }
    None
}

// --- Helper condivisi (analoghi a struttura.rs) ------------------------------

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
        Object::String(b, _) => {
            if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
                let u: Vec<u16> = b[2..]
                    .chunks(2)
                    .map(|c| ((c[0] as u16) << 8) | (*c.get(1).unwrap_or(&0) as u16))
                    .collect();
                Some(String::from_utf16_lossy(&u))
            } else {
                Some(b.iter().map(|&x| x as char).collect())
            }
        }
        _ => None,
    }
}
