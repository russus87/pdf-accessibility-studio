//! Lettura dei segnalibri (outline) del PDF tramite Pdfium, per un indice
//! navigabile: ogni voce ha titolo, pagina di destinazione e livello di
//! annidamento.

use std::path::Path;

use pdfium_render::prelude::PdfBookmark;
use serde::Serialize;

use crate::documento::con_documento;
use crate::errore::Risultato;

/// Una voce dell'indice/outline.
#[derive(Debug, Clone, Serialize)]
pub struct Segnalibro {
    pub titolo: String,
    /// Pagina di destinazione (0-based), se disponibile.
    pub pagina: Option<i32>,
    /// Livello di annidamento (0 = primo livello).
    pub livello: usize,
}

/// Restituisce l'elenco dei segnalibri in ordine di lettura (prefisso).
pub fn segnalibri(percorso: &Path) -> Risultato<Vec<Segnalibro>> {
    con_documento(percorso, |doc| {
        let mut out = Vec::new();
        let mut nodo = doc.bookmarks().root();
        while let Some(b) = nodo {
            visita(&b, 0, &mut out);
            nodo = b.next_sibling();
        }
        Ok(out)
    })
}

fn visita(b: &PdfBookmark, livello: usize, out: &mut Vec<Segnalibro>) {
    let titolo = b.title().unwrap_or_default();
    let pagina = b
        .destination()
        .and_then(|d| d.page_index().ok())
        .map(|p| p as i32);
    out.push(Segnalibro { titolo, pagina, livello });

    for figlio in b.iter_direct_children() {
        visita(&figlio, livello + 1, out);
    }
}
