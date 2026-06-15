//! pdfa-core: logica pura sui PDF di PDF Accessibility Studio.
//!
//! Moduli:
//! - `pdfium`: caricamento della libreria nativa Pdfium (una volta sola).
//! - `documento`: apertura PDF e rendering pagine in PNG.
//! - `errore`: tipo di errore unico del core.
//!
//! Le fasi successive aggiungeranno: validazione accessibilita', sintesi
//! vocale (ordine logico dei tag), confronto (testo/pixel/tag), export tag.

pub mod confronto;
pub mod contrasto;
pub mod correzione;
pub mod documento;
pub mod errore;
pub mod export;
pub mod geometria;
pub mod lettura;
pub mod metadati;
pub mod ocr;
pub mod pagine;
pub mod pdfium;
pub mod ricerca;
pub mod segnalibri;
pub mod struttura;
pub mod tts;
pub mod validazione;

pub use documento::{apri, render_pagina, rimuovi_dalla_cache, testo_pagine, InfoDocumento};
pub use errore::{Errore, Risultato};
pub use struttura::{analizza, InfoStruttura};
pub use validazione::{valida, Report};
