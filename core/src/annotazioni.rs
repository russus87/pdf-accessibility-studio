//! Annotazioni e revisione (Fase 38): evidenziazioni, note e caselle di testo
//! salvate come annotazioni PDF (/Annots), più un riepilogo esportabile.
//!
//! Coordinate in millimetri, origine in alto a sinistra (come i righelli).

use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Colore RGB 0-255.
#[derive(Debug, Clone, Copy)]
pub struct Colore {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Un'annotazione da aggiungere.
#[derive(Debug, Clone)]
pub enum Annot {
    /// Evidenziazione di un'area.
    Evidenzia { pagina: u16, x_mm: f32, y_mm: f32, larghezza_mm: f32, altezza_mm: f32, colore: Colore },
    /// Nota (icona con testo nel popup).
    Nota { pagina: u16, x_mm: f32, y_mm: f32, contenuto: String, colore: Colore },
    /// Casella di testo visibile (FreeText).
    Testo { pagina: u16, x_mm: f32, y_mm: f32, larghezza_mm: f32, altezza_mm: f32, contenuto: String, dim_pt: f32, colore: Colore },
}

/// Una voce del riepilogo annotazioni.
#[derive(Debug, Clone, Serialize)]
pub struct VoceRiepilogo {
    pub pagina: i32,
    pub tipo: String,
    pub contenuto: String,
}

fn pagina_di(a: &Annot) -> u16 {
    match a {
        Annot::Evidenzia { pagina, .. } | Annot::Nota { pagina, .. } | Annot::Testo { pagina, .. } => *pagina,
    }
}

/// Aggiunge le annotazioni e salva una copia. Ritorna quante ne sono state aggiunte.
pub fn applica(origine: &Path, destinazione: &Path, annotazioni: &[Annot]) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let mut pagine: std::collections::HashMap<u16, (ObjectId, f32)> = std::collections::HashMap::new();
    for (num, id) in doc.get_pages() {
        pagine.insert((num - 1) as u16, (id, altezza_pagina(&doc, id)));
    }

    let mut per_pagina: std::collections::HashMap<ObjectId, Vec<ObjectId>> = std::collections::HashMap::new();
    let mut count = 0;

    for a in annotazioni {
        let Some(&(page_id, h)) = pagine.get(&pagina_di(a)) else { continue };
        let dict = costruisci(a, page_id, h);
        let id = doc.add_object(dict);
        per_pagina.entry(page_id).or_default().push(id);
        count += 1;
    }

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

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(count)
}

fn costruisci(a: &Annot, page_id: ObjectId, h: f32) -> Dictionary {
    let mut d = Dictionary::new();
    d.set("Type", Object::Name(b"Annot".to_vec()));
    d.set("P", Object::Reference(page_id));
    d.set("F", Object::Integer(4)); // Print
    match a {
        Annot::Evidenzia { x_mm, y_mm, larghezza_mm, altezza_mm, colore, .. } => {
            let (x0, x1) = (mm(*x_mm), mm(*x_mm + *larghezza_mm));
            let (y1, y0) = (h - mm(*y_mm), h - mm(*y_mm + *altezza_mm));
            d.set("Subtype", Object::Name(b"Highlight".to_vec()));
            d.set("Rect", rett(x0, y0, x1, y1));
            d.set("QuadPoints", Object::Array(vec![
                reale(x0), reale(y1), reale(x1), reale(y1),
                reale(x0), reale(y0), reale(x1), reale(y0),
            ]));
            d.set("C", colore_pdf(colore));
        }
        Annot::Nota { x_mm, y_mm, contenuto, colore, .. } => {
            let x = mm(*x_mm);
            let y = h - mm(*y_mm);
            d.set("Subtype", Object::Name(b"Text".to_vec()));
            d.set("Rect", rett(x, y - 20.0, x + 20.0, y));
            d.set("Contents", stringa_utf16(contenuto));
            d.set("Name", Object::Name(b"Comment".to_vec()));
            d.set("Open", Object::Boolean(false));
            d.set("C", colore_pdf(colore));
        }
        Annot::Testo { x_mm, y_mm, larghezza_mm, altezza_mm, contenuto, dim_pt, colore, .. } => {
            let (x0, x1) = (mm(*x_mm), mm(*x_mm + *larghezza_mm));
            let (y1, y0) = (h - mm(*y_mm), h - mm(*y_mm + *altezza_mm));
            d.set("Subtype", Object::Name(b"FreeText".to_vec()));
            d.set("Rect", rett(x0, y0, x1, y1));
            d.set("Contents", stringa_utf16(contenuto));
            let da = format!("/Helv {dim_pt} Tf {:.3} {:.3} {:.3} rg", colore.r as f32 / 255.0, colore.g as f32 / 255.0, colore.b as f32 / 255.0);
            d.set("DA", Object::string_literal(da));
        }
    }
    d
}

/// Legge le annotazioni presenti e ne fa un riepilogo.
pub fn riepilogo(origine: &Path) -> Risultato<Vec<VoceRiepilogo>> {
    let doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;
    let mut out = Vec::new();
    for (num, page_id) in doc.get_pages() {
        let Ok(page) = doc.get_object(page_id).and_then(|o| o.as_dict()) else { continue };
        let annots = match page.get(b"Annots") {
            Ok(Object::Array(a)) => a.clone(),
            Ok(Object::Reference(id)) => doc.get_object(*id).and_then(|o| o.as_array()).cloned().unwrap_or_default(),
            _ => continue,
        };
        for a in annots {
            let Some(d) = a.as_reference().ok().and_then(|id| doc.get_object(id).ok()).and_then(|o| o.as_dict().ok()) else { continue };
            let tipo = d.get(b"Subtype").and_then(|o| o.as_name()).map(|n| String::from_utf8_lossy(n).into_owned()).unwrap_or_default();
            let contenuto = leggi_testo(&doc, d, b"Contents").unwrap_or_default();
            out.push(VoceRiepilogo { pagina: num as i32 - 1, tipo, contenuto });
        }
    }
    Ok(out)
}

// --- Helper ------------------------------------------------------------------

fn altezza_pagina(doc: &Document, page_id: ObjectId) -> f32 {
    fn mb(doc: &Document, id: ObjectId, prof: usize) -> Option<f32> {
        if prof > 16 { return None; }
        let d = doc.get_object(id).ok()?.as_dict().ok()?;
        if let Ok(Object::Array(mb)) = d.get(b"MediaBox") {
            if mb.len() == 4 { return Some((num(&mb[3]) - num(&mb[1])).abs()); }
        }
        let parent = d.get(b"Parent").ok()?.as_reference().ok()?;
        mb(doc, parent, prof + 1)
    }
    mb(doc, page_id, 0).unwrap_or(842.0)
}

fn num(o: &Object) -> f32 {
    match o { Object::Integer(i) => *i as f32, Object::Real(r) => *r, _ => 0.0 }
}
fn reale(v: f32) -> Object { Object::Real(v) }
fn rett(x0: f32, y0: f32, x1: f32, y1: f32) -> Object {
    Object::Array(vec![reale(x0), reale(y0), reale(x1), reale(y1)])
}
fn mm(v: f32) -> f32 { v * 72.0 / 25.4 }
fn colore_pdf(c: &Colore) -> Object {
    Object::Array(vec![reale(c.r as f32 / 255.0), reale(c.g as f32 / 255.0), reale(c.b as f32 / 255.0)])
}
fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() { bytes.extend_from_slice(&u.to_be_bytes()); }
    Object::String(bytes, StringFormat::Hexadecimal)
}
fn leggi_testo(doc: &Document, d: &Dictionary, chiave: &[u8]) -> Option<String> {
    let o = d.get(chiave).ok()?;
    let o = match o { Object::Reference(id) => doc.get_object(*id).ok()?, x => x };
    let Object::String(b, _) = o else { return None };
    if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
        let u: Vec<u16> = b[2..].chunks(2).map(|c| ((c[0] as u16) << 8) | (*c.get(1).unwrap_or(&0) as u16)).collect();
        Some(String::from_utf16_lossy(&u))
    } else {
        Some(b.iter().map(|&x| x as char).collect())
    }
}
