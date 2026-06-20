//! Aggiunta di **campi modulo editabili** a un PDF (Fase 26).
//!
//! Crea campi di testo AcroForm (widget) posizionati in millimetri (origine in
//! alto a sinistra, come i righelli dell'editor) e li registra nell'AcroForm del
//! documento, impostando `NeedAppearances` così i lettori ne disegnano l'aspetto.
//! Ogni campo riceve anche il tooltip `/TU` (etichetta accessibile, vedi Fase 20).
//! Salva sempre una copia.

use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};

use crate::errore::{Errore, Risultato};

/// Un nuovo campo di testo da inserire.
#[derive(Debug, Clone)]
pub struct NuovoCampo {
    pub pagina: u16,
    pub x_mm: f32,
    pub y_mm: f32,
    pub larghezza_mm: f32,
    pub altezza_mm: f32,
    /// Nome interno univoco del campo (/T).
    pub nome: String,
    /// Etichetta accessibile (/TU).
    pub tooltip: String,
    /// Valore iniziale del campo, se presente.
    pub valore: String,
}

/// Aggiunge i campi al PDF e salva una copia in `destinazione`. Ritorna quanti
/// campi sono stati creati.
pub fn aggiungi(origine: &Path, destinazione: &Path, campi: &[NuovoCampo]) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    // Indice pagina (0-based) -> ObjectId + altezza pagina in punti.
    let mut pagine: std::collections::HashMap<u16, (ObjectId, f32)> = std::collections::HashMap::new();
    for (num, id) in doc.get_pages() {
        let h = altezza_pagina(&doc, id);
        pagine.insert((num - 1) as u16, (id, h));
    }

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo non trovato".into()))?;

    let mut nuovi_field_ids: Vec<ObjectId> = Vec::new();
    let mut per_pagina: std::collections::HashMap<ObjectId, Vec<ObjectId>> = std::collections::HashMap::new();

    for c in campi {
        let Some(&(page_id, h)) = pagine.get(&c.pagina) else {
            continue;
        };
        // Rettangolo PDF (origine in basso a sinistra).
        let x0 = mm(c.x_mm);
        let x1 = mm(c.x_mm + c.larghezza_mm);
        let y1 = h - mm(c.y_mm);
        let y0 = h - mm(c.y_mm + c.altezza_mm);

        let mut w = Dictionary::new();
        w.set("Type", Object::Name(b"Annot".to_vec()));
        w.set("Subtype", Object::Name(b"Widget".to_vec()));
        w.set("FT", Object::Name(b"Tx".to_vec()));
        w.set("T", stringa_utf16(&c.nome));
        if !c.tooltip.trim().is_empty() {
            w.set("TU", stringa_utf16(&c.tooltip));
        }
        if !c.valore.is_empty() {
            w.set("V", stringa_utf16(&c.valore));
        }
        w.set(
            "Rect",
            Object::Array(vec![reale(x0), reale(y0), reale(x1), reale(y1)]),
        );
        w.set("P", Object::Reference(page_id));
        w.set("F", Object::Integer(4)); // Print
        w.set("DA", Object::string_literal("/Helv 0 Tf 0 g")); // 0 = auto-size
        let fid = doc.add_object(w);
        nuovi_field_ids.push(fid);
        per_pagina.entry(page_id).or_default().push(fid);
    }

    if nuovi_field_ids.is_empty() {
        return Ok(0);
    }

    // Aggiunge i widget agli /Annots delle rispettive pagine.
    for (page_id, ids) in per_pagina {
        let page = doc
            .get_object_mut(page_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("pagina: {e}")))?;
        let mut annots = match page.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        annots.extend(ids.into_iter().map(Object::Reference));
        page.set("Annots", Object::Array(annots));
    }

    // AcroForm: crea o aggiorna (Fields, NeedAppearances, DA, DR/font).
    aggiorna_acroform(&mut doc, catalog_id, &nuovi_field_ids)?;

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(nuovi_field_ids.len())
}

/// Crea/aggiorna il dizionario AcroForm con i nuovi campi e le risorse minime.
fn aggiorna_acroform(doc: &mut Document, catalog_id: ObjectId, nuovi: &[ObjectId]) -> Risultato<()> {
    // Recupera l'AcroForm esistente (dict diretto o riferimento) o creane uno.
    let acro_ref = doc
        .get_object(catalog_id)
        .and_then(|o| o.as_dict())
        .ok()
        .and_then(|c| c.get(b"AcroForm").ok())
        .and_then(|o| o.as_reference().ok());

    let acro_id = match acro_ref {
        Some(id) => id,
        None => {
            let id = doc.add_object(Dictionary::new());
            let cat = doc
                .get_object_mut(catalog_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;
            cat.set("AcroForm", Object::Reference(id));
            id
        }
    };

    // Font Helvetica per le risorse di default (DR), referenziato come /Helv nel DA.
    let mut helv = Dictionary::new();
    helv.set("Type", Object::Name(b"Font".to_vec()));
    helv.set("Subtype", Object::Name(b"Type1".to_vec()));
    helv.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    let helv_id = doc.add_object(helv);

    let acro = doc
        .get_object_mut(acro_id)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| Errore::Pdfium(format!("AcroForm: {e}")))?;

    let mut fields = match acro.get(b"Fields") {
        Ok(Object::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    fields.extend(nuovi.iter().map(|id| Object::Reference(*id)));
    acro.set("Fields", Object::Array(fields));
    acro.set("NeedAppearances", Object::Boolean(true));
    acro.set("DA", Object::string_literal("/Helv 0 Tf 0 g"));

    let mut font_dr = Dictionary::new();
    font_dr.set("Helv", Object::Reference(helv_id));
    let mut dr = Dictionary::new();
    dr.set("Font", Object::Dictionary(font_dr));
    acro.set("DR", Object::Dictionary(dr));
    Ok(())
}

/// Altezza pagina in punti dal MediaBox (con fallback A4).
fn altezza_pagina(doc: &Document, page_id: ObjectId) -> f32 {
    fn media_box(doc: &Document, id: ObjectId, prof: usize) -> Option<f32> {
        if prof > 16 {
            return None;
        }
        let d = doc.get_object(id).ok()?.as_dict().ok()?;
        if let Ok(Object::Array(mb)) = d.get(b"MediaBox") {
            if mb.len() == 4 {
                let y0 = numero(&mb[1]);
                let y1 = numero(&mb[3]);
                return Some((y1 - y0).abs());
            }
        }
        // Eredita dal genitore Pages.
        let parent = d.get(b"Parent").ok()?.as_reference().ok()?;
        media_box(doc, parent, prof + 1)
    }
    media_box(doc, page_id, 0).unwrap_or(842.0)
}

fn numero(o: &Object) -> f32 {
    match o {
        Object::Integer(i) => *i as f32,
        Object::Real(r) => *r,
        _ => 0.0,
    }
}

fn reale(v: f32) -> Object {
    Object::Real(v)
}

/// Millimetri -> punti PDF.
fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}

/// Codifica una stringa come testo PDF UTF-16BE.
fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
