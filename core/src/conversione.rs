//! Conversione tra PDF e immagini (Fase 29).
//!
//! - `pdf_a_immagini`: renderizza ogni pagina e ne ritorna i byte PNG o JPEG.
//! - `immagini_a_pdf`: crea un PDF con un'immagine per pagina (pagina dimensionata
//!   all'immagine), tramite Pdfium.

use std::io::Cursor;
use std::path::Path;

use image::DynamicImage;
use pdfium_render::prelude::*;

use crate::errore::{Errore, Risultato};

/// Renderizza ogni pagina del PDF alla larghezza data e ritorna i byte immagine
/// (PNG, oppure JPEG se `jpeg` è true) pagina per pagina.
pub fn pdf_a_immagini(origine: &Path, larghezza_px: i32, jpeg: bool) -> Risultato<Vec<Vec<u8>>> {
    let info = crate::apri(origine)?;
    let mut out = Vec::with_capacity(info.pagine as usize);
    for i in 0..info.pagine as i32 {
        let png = crate::render_pagina(origine, i, larghezza_px)?;
        if jpeg {
            let img = image::load_from_memory(&png)
                .map_err(|e| Errore::Io(format!("decodifica pagina {i}: {e}")))?;
            let mut buf = Cursor::new(Vec::new());
            DynamicImage::ImageRgb8(img.to_rgb8())
                .write_to(&mut buf, image::ImageFormat::Jpeg)
                .map_err(|e| Errore::Io(format!("JPEG pagina {i}: {e}")))?;
            out.push(buf.into_inner());
        } else {
            out.push(png);
        }
    }
    Ok(out)
}

/// Crea un PDF con un'immagine per pagina e lo salva. Ogni pagina è dimensionata
/// all'immagine (a 96 DPI). Ritorna il numero di pagine create.
pub fn immagini_a_pdf(immagini: &[Vec<u8>], destinazione: &Path) -> Risultato<usize> {
    let pdfium = crate::pdfium::istanza()?;
    let mut doc = pdfium
        .create_new_pdf()
        .map_err(|e| Errore::Pdfium(format!("nuovo PDF: {e}")))?;

    let mut n = 0;
    for (i, bytes) in immagini.iter().enumerate() {
        let img = image::load_from_memory(bytes)
            .map_err(|e| Errore::Io(format!("immagine {i}: {e}")))?;
        // px -> punti a 96 DPI.
        let wpt = img.width() as f32 * 72.0 / 96.0;
        let hpt = img.height() as f32 * 72.0 / 96.0;
        let size = PdfPagePaperSize::from_points(PdfPoints::new(wpt), PdfPoints::new(hpt));
        let mut page = doc
            .pages_mut()
            .create_page_at_end(size)
            .map_err(|e| Errore::Pdfium(format!("pagina {i}: {e}")))?;
        page.objects_mut()
            .create_image_object(
                PdfPoints::ZERO,
                PdfPoints::ZERO,
                &img,
                Some(PdfPoints::new(wpt)),
                Some(PdfPoints::new(hpt)),
            )
            .map_err(|e| Errore::Pdfium(format!("immagine pagina {i}: {e}")))?;
        page.regenerate_content()
            .map_err(|e| Errore::Pdfium(format!("rigenerazione {i}: {e}")))?;
        n += 1;
    }

    doc.save_to_file(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(n)
}
