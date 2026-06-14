//! Apertura dei PDF, cache dei documenti e rendering delle pagine in PNG.
//!
//! I documenti aperti con Pdfium vengono tenuti in una cache globale: aprire un
//! PDF e renderizzarne le pagine non richiede di rileggere il file ogni volta
//! (utile per i thumbnail e lo scorrimento). `PdfDocument` e' Send+Sync, quindi
//! e' condivisibile tra i thread di Tauri.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use pdfium_render::prelude::{PdfDocument, PdfDocumentMetadataTagType, PdfRenderConfig};
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

/// Cache globale dei documenti Pdfium aperti, indicizzata per percorso.
static CACHE: OnceLock<Mutex<HashMap<PathBuf, PdfDocument<'static>>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<PathBuf, PdfDocument<'static>>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Esegue `f` su un documento, caricandolo nella cache se assente. Il lock e'
/// tenuto per tutta l'operazione: va bene perche' Pdfium serializza comunque le
/// chiamate (feature `thread_safe`).
fn con_documento<R>(percorso: &Path, f: impl FnOnce(&PdfDocument) -> Risultato<R>) -> Risultato<R> {
    let mut mappa = cache().lock().unwrap();
    if !mappa.contains_key(percorso) {
        let doc = istanza()?.load_pdf_from_file(percorso, None)?;
        mappa.insert(percorso.to_path_buf(), doc);
    }
    f(mappa.get(percorso).unwrap())
}

/// Rimuove un documento dalla cache (quando si chiude l'ultima scheda che lo usa).
pub fn rimuovi_dalla_cache(percorso: &Path) {
    cache().lock().unwrap().remove(percorso);
}

/// Apre il PDF e ne legge le informazioni di base (numero pagine, titolo).
pub fn apri(percorso: &Path) -> Risultato<InfoDocumento> {
    con_documento(percorso, |doc| {
        let pagine = doc.pages().len() as u16;
        let titolo = doc
            .metadata()
            .get(PdfDocumentMetadataTagType::Title)
            .map(|tag| tag.value().to_string())
            .filter(|s| !s.trim().is_empty());
        Ok(InfoDocumento { pagine, titolo })
    })
}

/// Renderizza una pagina (indice 0-based) a una data larghezza in pixel e
/// restituisce i byte PNG. L'altezza segue le proporzioni della pagina.
pub fn render_pagina(percorso: &Path, indice: i32, larghezza: i32) -> Risultato<Vec<u8>> {
    let immagine = render_immagine(percorso, indice, larghezza)?;
    let mut buffer = Vec::new();
    immagine.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)?;
    Ok(buffer)
}

/// Come `render_pagina` ma restituisce l'immagine grezza (per il confronto
/// pixel-to-pixel, Fase 4).
pub fn render_immagine(percorso: &Path, indice: i32, larghezza: i32) -> Risultato<image::DynamicImage> {
    con_documento(percorso, |doc| {
        let pagina = doc.pages().get(indice)?;
        let config = PdfRenderConfig::new().set_target_width(larghezza);
        let bitmap = pagina.render_with_config(&config)?;
        Ok(bitmap.as_image()?)
    })
}

/// Estrae il testo di ogni pagina (una stringa per pagina), in ordine di
/// pagina. E' la base per la lettura vocale (Fase 3) e il confronto testuale
/// (Fase 4).
pub fn testo_pagine(percorso: &Path) -> Risultato<Vec<String>> {
    con_documento(percorso, |doc| {
        let mut pagine = Vec::new();
        for pagina in doc.pages().iter() {
            let testo = pagina.text().map(|t| t.all()).unwrap_or_default();
            pagine.push(testo);
        }
        Ok(pagine)
    })
}
