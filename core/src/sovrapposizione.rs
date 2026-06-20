//! Sovrapposizione di elementi su un PDF esistente (Fase 24): testo, immagini e
//! filigrana. Usa Pdfium per aggiungere oggetti pagina nativi (font e immagini
//! gestiti dalla libreria) e salva una copia.
//!
//! Coordinate: in **millimetri** con origine in **alto a sinistra** (come i
//! righelli dell'editor). Internamente vengono convertite nel sistema PDF
//! (origine in basso a sinistra, unita' in punti).

use image::DynamicImage;
use pdfium_render::prelude::*;

use crate::errore::{Errore, Risultato};

/// Colore RGB (0-255).
#[derive(Debug, Clone, Copy)]
pub struct Colore {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Colore {
    fn default() -> Self {
        Colore { r: 0, g: 0, b: 0 }
    }
}

/// Un elemento da sovrapporre a una pagina specifica.
#[derive(Debug, Clone)]
pub enum Elemento {
    Testo {
        pagina: u16,
        x_mm: f32,
        y_mm: f32,
        testo: String,
        dim_pt: f32,
        colore: Colore,
        /// Opacita' 0-255 (255 = opaco).
        opacita: u8,
        /// Rotazione oraria in gradi.
        rotazione: f32,
    },
    Immagine {
        pagina: u16,
        x_mm: f32,
        y_mm: f32,
        larghezza_mm: f32,
        altezza_mm: f32,
        /// Byte del PNG.
        png: Vec<u8>,
        opacita: u8,
        rotazione: f32,
    },
}

/// Filigrana applicata a **tutte** le pagine, centrata.
#[derive(Debug, Clone)]
pub enum Filigrana {
    Testo {
        testo: String,
        dim_pt: f32,
        colore: Colore,
        opacita: u8,
        rotazione: f32,
    },
    Immagine {
        png: Vec<u8>,
        larghezza_mm: f32,
        altezza_mm: f32,
        opacita: u8,
        rotazione: f32,
    },
}

/// Applica gli elementi e l'eventuale filigrana a `origine`, salvando in
/// `destinazione`. Ritorna il numero di oggetti aggiunti.
pub fn applica(
    origine: &std::path::Path,
    destinazione: &std::path::Path,
    elementi: &[Elemento],
    filigrana: Option<&Filigrana>,
) -> Risultato<usize> {
    let pdfium = crate::pdfium::istanza()?;
    let mut doc = pdfium
        .load_pdf_from_file(origine, None)
        .map_err(|e| Errore::Pdfium(format!("apertura: {e}")))?;

    // Token del font (Helvetica) ricavato prima del prestito delle pagine.
    let font = doc.fonts_mut().helvetica();

    let n_pagine = doc.pages().len();
    let mut aggiunti = 0;

    for i in 0..n_pagine {
        let mut page = doc
            .pages()
            .get(i)
            .map_err(|e| Errore::Pdfium(format!("pagina {i}: {e}")))?;
        let h = page.height().value;
        let w = page.width().value;

        // Elementi assegnati a questa pagina.
        for el in elementi.iter().filter(|e| pagina_di(e) as i32 == i) {
            aggiungi_elemento(&mut page, font, el, h)?;
            aggiunti += 1;
        }

        // Filigrana centrata su ogni pagina.
        if let Some(f) = filigrana {
            aggiungi_filigrana(&mut page, font, f, w, h)?;
            aggiunti += 1;
        }

        page.regenerate_content()
            .map_err(|e| Errore::Pdfium(format!("rigenerazione pagina {i}: {e}")))?;
    }

    doc.save_to_file(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(aggiunti)
}

fn pagina_di(e: &Elemento) -> u16 {
    match e {
        Elemento::Testo { pagina, .. } | Elemento::Immagine { pagina, .. } => *pagina,
    }
}

/// Aggiunge un singolo elemento alla pagina (coordinate top-left in mm).
fn aggiungi_elemento(
    page: &mut PdfPage,
    font: PdfFontToken,
    el: &Elemento,
    h_pt: f32,
) -> Risultato<()> {
    match el {
        Elemento::Testo { x_mm, y_mm, testo, dim_pt, colore, opacita, rotazione, .. } => {
            let x = PdfPoints::from_mm(*x_mm);
            // top-left -> bottom-left: la baseline scende di dim_pt.
            let y = PdfPoints::new(h_pt - mm(*y_mm) - dim_pt);
            let mut obj = page
                .objects_mut()
                .create_text_object(x, y, testo, font, PdfPoints::new(*dim_pt))
                .map_err(|e| Errore::Pdfium(format!("testo: {e}")))?;
            obj.set_fill_color(PdfColor::new(colore.r, colore.g, colore.b, *opacita))
                .map_err(|e| Errore::Pdfium(format!("colore testo: {e}")))?;
            if *rotazione != 0.0 {
                obj.rotate_clockwise_degrees(*rotazione)
                    .map_err(|e| Errore::Pdfium(format!("rotazione testo: {e}")))?;
            }
        }
        Elemento::Immagine { x_mm, y_mm, larghezza_mm, altezza_mm, png, opacita, rotazione, .. } => {
            let img = immagine_con_opacita(png, *opacita)?;
            let x = PdfPoints::from_mm(*x_mm);
            // top-left -> bottom-left: l'origine immagine e' in basso.
            let y = PdfPoints::new(h_pt - mm(*y_mm) - mm(*altezza_mm));
            let mut obj = page
                .objects_mut()
                .create_image_object(
                    x,
                    y,
                    &img,
                    Some(PdfPoints::from_mm(*larghezza_mm)),
                    Some(PdfPoints::from_mm(*altezza_mm)),
                )
                .map_err(|e| Errore::Pdfium(format!("immagine: {e}")))?;
            if *rotazione != 0.0 {
                obj.rotate_clockwise_degrees(*rotazione)
                    .map_err(|e| Errore::Pdfium(format!("rotazione immagine: {e}")))?;
            }
        }
    }
    Ok(())
}

/// Aggiunge la filigrana centrata sulla pagina (larghezza/altezza in punti).
fn aggiungi_filigrana(
    page: &mut PdfPage,
    font: PdfFontToken,
    f: &Filigrana,
    w_pt: f32,
    h_pt: f32,
) -> Risultato<()> {
    match f {
        Filigrana::Testo { testo, dim_pt, colore, opacita, rotazione } => {
            // Stima larghezza testo per centrare (Helvetica ~0.5em a carattere).
            let larghezza_stim = testo.chars().count() as f32 * dim_pt * 0.5;
            let x = PdfPoints::new((w_pt - larghezza_stim).max(0.0) / 2.0);
            let y = PdfPoints::new(h_pt / 2.0);
            let mut obj = page
                .objects_mut()
                .create_text_object(x, y, testo, font, PdfPoints::new(*dim_pt))
                .map_err(|e| Errore::Pdfium(format!("filigrana testo: {e}")))?;
            obj.set_fill_color(PdfColor::new(colore.r, colore.g, colore.b, *opacita))
                .map_err(|e| Errore::Pdfium(format!("colore filigrana: {e}")))?;
            if *rotazione != 0.0 {
                obj.rotate_clockwise_degrees(*rotazione)
                    .map_err(|e| Errore::Pdfium(format!("rotazione filigrana: {e}")))?;
            }
        }
        Filigrana::Immagine { png, larghezza_mm, altezza_mm, opacita, rotazione } => {
            let img = immagine_con_opacita(png, *opacita)?;
            let lw = mm(*larghezza_mm);
            let lh = mm(*altezza_mm);
            let x = PdfPoints::new((w_pt - lw).max(0.0) / 2.0);
            let y = PdfPoints::new((h_pt - lh).max(0.0) / 2.0);
            let mut obj = page
                .objects_mut()
                .create_image_object(
                    x,
                    y,
                    &img,
                    Some(PdfPoints::from_mm(*larghezza_mm)),
                    Some(PdfPoints::from_mm(*altezza_mm)),
                )
                .map_err(|e| Errore::Pdfium(format!("filigrana immagine: {e}")))?;
            if *rotazione != 0.0 {
                obj.rotate_clockwise_degrees(*rotazione)
                    .map_err(|e| Errore::Pdfium(format!("rotazione filigrana img: {e}")))?;
            }
        }
    }
    Ok(())
}

/// Decodifica un PNG e ne scala il canale alfa secondo l'opacita' (0-255).
fn immagine_con_opacita(png: &[u8], opacita: u8) -> Risultato<DynamicImage> {
    let mut img = image::load_from_memory(png)
        .map_err(|e| Errore::Io(format!("immagine non valida: {e}")))?
        .to_rgba8();
    if opacita < 255 {
        for p in img.pixels_mut() {
            p.0[3] = ((p.0[3] as u16 * opacita as u16) / 255) as u8;
        }
    }
    Ok(DynamicImage::ImageRgba8(img))
}

/// Millimetri -> punti PDF (1 mm = 72/25.4 pt).
fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}
