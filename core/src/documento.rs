//! Apertura dei PDF e rendering delle pagine in PNG.
//!
//! Per la Fase 1 riapriamo il file ad ogni operazione: e' semplice e robusto
//! (niente strutture auto-referenziali con i lifetime di Pdfium). Una cache dei
//! documenti aperti potra' arrivare in una fase successiva se servira'.

use std::io::Cursor;
use std::path::Path;

use pdfium_render::prelude::{PdfDocumentMetadataTagType, PdfRenderConfig};
use serde::Serialize;

use crate::errore::Risultato;
use crate::pdfium::istanza;

/// Informazioni di base su un PDF appena aperto, mostrate nella UI.
#[derive(Debug, Clone, Serialize)]
pub struct InfoDocumento {
    /// Numero di pagine.
    pub pagine: u16,
    /// Titolo dai metadati, se presente (utile per il nome della scheda).
    pub titolo: Option<String>,
}

/// Apre il PDF e ne legge le informazioni di base (numero pagine, titolo).
pub fn apri(percorso: &Path) -> Risultato<InfoDocumento> {
    let pdfium = istanza()?;
    let doc = pdfium.load_pdf_from_file(percorso, None)?;

    let pagine = doc.pages().len() as u16;
    let titolo = doc
        .metadata()
        .get(PdfDocumentMetadataTagType::Title)
        .map(|tag| tag.value().to_string())
        .filter(|s| !s.trim().is_empty());

    Ok(InfoDocumento { pagine, titolo })
}

/// Renderizza una pagina (indice 0-based) a una data larghezza in pixel e
/// restituisce i byte PNG. L'altezza segue le proporzioni della pagina.
pub fn render_pagina(percorso: &Path, indice: i32, larghezza: i32) -> Risultato<Vec<u8>> {
    let pdfium = istanza()?;
    let doc = pdfium.load_pdf_from_file(percorso, None)?;
    let pagina = doc.pages().get(indice)?;

    let config = PdfRenderConfig::new().set_target_width(larghezza);
    let immagine = pagina.render_with_config(&config)?.as_image()?;

    let mut buffer = Vec::new();
    immagine.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)?;
    Ok(buffer)
}
