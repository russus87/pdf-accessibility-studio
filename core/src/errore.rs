//! Tipo di errore unico del core, cosi' Tauri puo' restituirlo come stringa.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Errore {
    #[error("Pdfium non inizializzato: libreria nativa non trovata")]
    PdfiumMancante,

    #[error("errore Pdfium: {0}")]
    Pdfium(String),

    #[error("errore immagine: {0}")]
    Immagine(String),

    #[error("errore I/O: {0}")]
    Io(String),
}

// pdfium-render ha un suo tipo di errore: lo trasformiamo in stringa.
impl From<pdfium_render::prelude::PdfiumError> for Errore {
    fn from(e: pdfium_render::prelude::PdfiumError) -> Self {
        Errore::Pdfium(format!("{e:?}"))
    }
}

impl From<image::ImageError> for Errore {
    fn from(e: image::ImageError) -> Self {
        Errore::Immagine(e.to_string())
    }
}

impl From<std::io::Error> for Errore {
    fn from(e: std::io::Error) -> Self {
        Errore::Io(e.to_string())
    }
}

pub type Risultato<T> = std::result::Result<T, Errore>;
