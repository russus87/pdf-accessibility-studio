//! pdfa-core: logica pura sui PDF di PDF Accessibility Studio.
//!
//! Moduli:
//! - `pdfium`: caricamento della libreria nativa Pdfium (una volta sola).
//! - `documento`: apertura PDF e rendering pagine in PNG.
//! - `errore`: tipo di errore unico del core.
//!
//! Le fasi successive aggiungeranno: validazione accessibilita', sintesi
//! vocale (ordine logico dei tag), confronto (testo/pixel/tag), export tag.

pub mod documento;
pub mod errore;
pub mod pdfium;

pub use documento::{apri, render_pagina, InfoDocumento};
pub use errore::{Errore, Risultato};
