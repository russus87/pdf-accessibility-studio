//! pdfa-core: logica pura sui PDF di PDF Accessibility Studio.
//!
//! Moduli:
//! - `pdfium`: caricamento della libreria nativa Pdfium (una volta sola).
//! - `documento`: apertura PDF e rendering pagine in PNG.
//! - `errore`: tipo di errore unico del core.
//!
//! Le fasi successive aggiungeranno: validazione accessibilita', sintesi
//! vocale (ordine logico dei tag), confronto (testo/pixel/tag), export tag.

pub mod annotazioni;
pub mod artifact;
pub mod campi;
pub mod cifratura;
pub mod confronto;
pub mod contrasto;
pub mod conversione;
pub mod correzione;
pub mod doclang;
pub mod documento;
pub mod errore;
pub mod estrazione;
pub mod export;
pub mod form;
pub mod geometria;
pub mod intestazioni;
pub mod lettura;
pub mod metadati;
pub mod modello;
pub mod office;
pub mod ottimizzazione;
pub mod redazione;
pub mod ocr;
pub mod pagine;
pub mod pdfium;
pub mod ricerca;
pub mod segnalibri;
pub mod sovrapposizione;
pub mod struttura;
pub mod tabelle;
pub mod toc;
pub mod taggatura;
pub mod tts;
pub mod validazione;

pub use doclang::{analizza_json as analizza_doclang, Analisi as AnalisiDoclang};
pub use documento::{apri, dimensioni_pagina, render_pagina, rimuovi_dalla_cache, testo_pagine, InfoDocumento};
pub use errore::{Errore, Risultato};
pub use struttura::{analizza, InfoStruttura};
pub use validazione::{valida, Report};
