//! Ottimizzazione/compressione dei PDF (Fase 31).
//!
//! Riduce la dimensione del file: rimuove gli oggetti non referenziati, elimina
//! gli stream vuoti, comprime i content stream (Flate) e risalva usando object
//! stream + xref stream. Ritorna le dimensioni prima/dopo.

use std::path::Path;

use lopdf::{Document, SaveOptions};

use crate::errore::{Errore, Risultato};

/// Ottimizza `origine` e salva in `destinazione`. Ritorna (byte prima, byte dopo).
pub fn ottimizza(origine: &Path, destinazione: &Path) -> Risultato<(u64, u64)> {
    let prima = std::fs::metadata(origine).map(|m| m.len()).unwrap_or(0);

    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;
    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.compress();

    let opzioni = SaveOptions::builder()
        .use_object_streams(true)
        .use_xref_streams(true)
        .compression_level(9)
        .build();

    let mut file = std::fs::File::create(destinazione).map_err(|e| Errore::Io(format!("creazione: {e}")))?;
    doc.save_with_options(&mut file, opzioni)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;

    let dopo = std::fs::metadata(destinazione).map(|m| m.len()).unwrap_or(0);
    Ok((prima, dopo))
}
