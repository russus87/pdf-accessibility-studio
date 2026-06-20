//! Redazione (Fase 28): oscura **definitivamente** aree di una pagina rimuovendo
//! gli oggetti (testo/immagini) che le intersecano e coprendole con un rettangolo
//! nero pieno. A differenza di una semplice copertura, il contenuto sottostante
//! viene rimosso, quindi non è più recuperabile.
//!
//! Coordinate in millimetri, origine in alto a sinistra (come i righelli).

use std::path::Path;

use image::DynamicImage;
use pdfium_render::prelude::*;

use crate::errore::{Errore, Risultato};

/// Un'area da redigere.
#[derive(Debug, Clone)]
pub struct Area {
    pub pagina: u16,
    pub x_mm: f32,
    pub y_mm: f32,
    pub larghezza_mm: f32,
    pub altezza_mm: f32,
}

/// Redige le aree indicate e salva una copia. Ritorna quante aree sono state applicate.
pub fn redigi(origine: &Path, destinazione: &Path, aree: &[Area]) -> Risultato<usize> {
    let pdfium = crate::pdfium::istanza()?;
    let doc = pdfium
        .load_pdf_from_file(origine, None)
        .map_err(|e| Errore::Pdfium(format!("apertura: {e}")))?;

    // Tassello nero usato per coprire le aree (l'immagine viene scalata all'area).
    let nero = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([0, 0, 0])));

    let n_pagine = doc.pages().len();
    let mut tot = 0;

    for i in 0..n_pagine {
        let aree_pag: Vec<&Area> = aree.iter().filter(|a| a.pagina as i32 == i).collect();
        if aree_pag.is_empty() {
            continue;
        }

        let mut page = doc
            .pages()
            .get(i)
            .map_err(|e| Errore::Pdfium(format!("pagina {i}: {e}")))?;
        let h = page.height().value;

        // Rettangoli di redazione in coordinate PDF: (left, bottom, right, top).
        let rects: Vec<(f32, f32, f32, f32)> = aree_pag
            .iter()
            .map(|a| {
                let l = mm(a.x_mm);
                let r = mm(a.x_mm + a.larghezza_mm);
                let top = h - mm(a.y_mm);
                let bot = h - mm(a.y_mm + a.altezza_mm);
                (l, bot, r, top)
            })
            .collect();

        // Raccoglie gli indici degli oggetti che intersecano un'area.
        let mut da_rimuovere: Vec<usize> = Vec::new();
        {
            let objs = page.objects();
            for (idx, obj) in objs.iter().enumerate() {
                if let Ok(b) = obj.bounds() {
                    let (ol, ob, orr, ot) = (b.left().value, b.bottom().value, b.right().value, b.top().value);
                    if rects.iter().any(|&(l, bt, r, tp)| interseca(l, bt, r, tp, ol, ob, orr, ot)) {
                        da_rimuovere.push(idx);
                    }
                }
            }
        }

        // Rimuove dagli indici più alti ai più bassi per non invalidare gli indici.
        // Gli oggetti rimossi restano vivi fino a dopo il salvataggio per evitare
        // un uso-dopo-rilascio nei riferimenti interni di Pdfium.
        let mut rimossi = Vec::new();
        for &idx in da_rimuovere.iter().rev() {
            if let Ok(o) = page.objects_mut().remove_object_at_index(idx) {
                rimossi.push(o);
            }
        }
        std::mem::forget(rimossi);

        // Copre ogni area con un tassello nero (immagine scalata all'area).
        for &(l, bot, r, top) in &rects {
            page.objects_mut()
                .create_image_object(
                    PdfPoints::new(l),
                    PdfPoints::new(bot),
                    &nero,
                    Some(PdfPoints::new(r - l)),
                    Some(PdfPoints::new(top - bot)),
                )
                .map_err(|e| Errore::Pdfium(format!("copertura: {e}")))?;
            tot += 1;
        }

        page.regenerate_content()
            .map_err(|e| Errore::Pdfium(format!("rigenerazione {i}: {e}")))?;
    }

    doc.save_to_file(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(tot)
}

/// Test di intersezione tra due rettangoli (AABB).
fn interseca(l1: f32, b1: f32, r1: f32, t1: f32, l2: f32, b2: f32, r2: f32, t2: f32) -> bool {
    l1 < r2 && r1 > l2 && b1 < t2 && t1 > b2
}

fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}
