//! Stato condiviso dell'app: i documenti PDF attualmente aperti.
//!
//! Ad ogni PDF aperto associamo un id; la UI usa l'id per chiedere il rendering
//! delle pagine. Per ora memorizziamo solo il percorso del file (il rendering
//! riapre il file su richiesta — vedi `pdfa-core`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub struct StatoApp {
    /// id documento -> percorso del file PDF.
    pub documenti: Mutex<HashMap<String, PathBuf>>,
    contatore: AtomicU64,
}

impl StatoApp {
    /// Genera un id progressivo univoco per una nuova scheda.
    pub fn nuovo_id(&self) -> String {
        let n = self.contatore.fetch_add(1, Ordering::Relaxed);
        format!("doc{n}")
    }
}
