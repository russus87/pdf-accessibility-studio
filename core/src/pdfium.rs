//! Inizializzazione e accesso globale alla libreria Pdfium.
//!
//! Pdfium e' una libreria nativa (libpdfium.so/.dll/.dylib): va caricata una
//! volta sola. La teniamo in uno stato globale (`OnceLock`) cosi' tutti i
//! comandi Tauri la condividono. Con la feature "thread_safe" del crate le
//! chiamate sono serializzate, quindi e' sicura da piu' thread.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pdfium_render::prelude::Pdfium;

use crate::errore::{Errore, Risultato};

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

/// Prova a caricare Pdfium cercando la libreria nelle cartelle indicate (in
/// ordine) e infine tra le librerie di sistema. Va chiamata una volta sola
/// all'avvio dell'app; le chiamate successive non hanno effetto.
pub fn inizializza(cartelle: &[PathBuf]) -> Risultato<()> {
    if PDFIUM.get().is_some() {
        return Ok(());
    }

    // Prova ogni cartella candidata: file della libreria con nome di piattaforma.
    let mut bindings = None;
    for cartella in cartelle {
        let percorso = Pdfium::pdfium_platform_library_name_at_path(cartella);
        if let Ok(b) = Pdfium::bind_to_library(&percorso) {
            bindings = Some(b);
            break;
        }
    }

    // Ultimo tentativo: libreria di sistema (es. /usr/lib/libpdfium.so).
    let bindings = match bindings {
        Some(b) => b,
        None => Pdfium::bind_to_system_library().map_err(|_| Errore::PdfiumMancante)?,
    };

    // Se qualcun altro l'ha gia' inizializzata nel frattempo, va bene lo stesso.
    let _ = PDFIUM.set(Pdfium::new(bindings));
    Ok(())
}

/// Riferimento alla Pdfium globale, o errore se non e' stata inizializzata.
pub fn istanza() -> Risultato<&'static Pdfium> {
    PDFIUM.get().ok_or(Errore::PdfiumMancante)
}

/// Helper: nome del file libreria atteso per la piattaforma corrente
/// (utile a script e messaggi diagnostici).
pub fn nome_libreria() -> String {
    Pdfium::pdfium_platform_library_name_at_path(Path::new("."))
        .to_string_lossy()
        .trim_start_matches("./")
        .to_string()
}
