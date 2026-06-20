//! Numerazione pagine e intestazioni/piè di pagina automatici (Fase 30).
//!
//! Applica un testo a tutte le pagine in una delle sei posizioni standard
//! (alto/basso × sinistra/centro/destra). Il testo può contenere i segnaposto
//! `{n}` (numero pagina) e `{tot}` (totale). Si appoggia al motore di overlay.

use std::path::Path;

use crate::errore::Risultato;
use crate::sovrapposizione::{applica as overlay, Colore, Elemento};

const MM_PER_PT: f32 = 25.4 / 72.0;

/// Posizione del testo sulla pagina.
#[derive(Debug, Clone, Copy)]
pub enum Ancora {
    AltoSinistra,
    AltoCentro,
    AltoDestra,
    BassoSinistra,
    BassoCentro,
    BassoDestra,
}

impl Ancora {
    /// Interpreta una stringa ("alto-centro", "basso-destra", ...).
    pub fn da_str(s: &str) -> Ancora {
        match s {
            "alto-sinistra" => Ancora::AltoSinistra,
            "alto-centro" => Ancora::AltoCentro,
            "alto-destra" => Ancora::AltoDestra,
            "basso-sinistra" => Ancora::BassoSinistra,
            "basso-destra" => Ancora::BassoDestra,
            _ => Ancora::BassoCentro,
        }
    }
    fn in_alto(&self) -> bool {
        matches!(self, Ancora::AltoSinistra | Ancora::AltoCentro | Ancora::AltoDestra)
    }
}

/// Opzioni dell'intestazione/numerazione.
#[derive(Debug, Clone)]
pub struct Opzioni {
    pub testo: String,
    pub dim_pt: f32,
    pub colore: Colore,
    pub margine_mm: f32,
    pub ancora: Ancora,
    /// Numero da cui parte la numerazione (di solito 1).
    pub inizio_numerazione: i64,
}

/// Applica il testo a tutte le pagine e salva una copia. Ritorna quante pagine.
pub fn applica(origine: &Path, destinazione: &Path, op: &Opzioni) -> Risultato<usize> {
    let info = crate::apri(origine)?;
    let tot = info.pagine;

    let mut elementi = Vec::with_capacity(tot as usize);
    for i in 0..tot as i32 {
        let (wpt, hpt) = crate::dimensioni_pagina(origine, i)?;
        let w_mm = wpt * MM_PER_PT;
        let h_mm = hpt * MM_PER_PT;

        let numero = i as i64 + op.inizio_numerazione;
        let testo = op
            .testo
            .replace("{n}", &numero.to_string())
            .replace("{tot}", &tot.to_string());

        // Stima larghezza testo (Helvetica ~0.5em per carattere).
        let tw_mm = testo.chars().count() as f32 * op.dim_pt * 0.5 * MM_PER_PT;
        let dim_mm = op.dim_pt * MM_PER_PT;
        let m = op.margine_mm;

        let x_mm = match op.ancora {
            Ancora::AltoSinistra | Ancora::BassoSinistra => m,
            Ancora::AltoCentro | Ancora::BassoCentro => ((w_mm - tw_mm) / 2.0).max(0.0),
            Ancora::AltoDestra | Ancora::BassoDestra => (w_mm - m - tw_mm).max(0.0),
        };
        let y_mm = if op.ancora.in_alto() { m } else { (h_mm - m - dim_mm).max(0.0) };

        elementi.push(Elemento::Testo {
            pagina: i as u16,
            x_mm,
            y_mm,
            testo,
            dim_pt: op.dim_pt,
            colore: op.colore,
            opacita: 255,
            rotazione: 0.0,
        });
    }

    overlay(origine, destinazione, &elementi, None)?;
    Ok(tot as usize)
}
