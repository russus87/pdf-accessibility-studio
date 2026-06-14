//! Contrasto dei colori secondo WCAG 2.x.
//!
//! `rapporto` calcola il rapporto di contrasto esatto tra due colori.
//! `analizza_pagina` stima i due colori dominanti (sfondo e testo) di una pagina
//! renderizzata e ne valuta il contrasto: utile come screening, non come audit
//! per singolo elemento.

use std::collections::HashMap;

use serde::Serialize;

use crate::errore::Risultato;

/// Esito del contrasto di una pagina.
#[derive(Debug, Clone, Serialize)]
pub struct ContrastoPagina {
    pub pagina: i32,
    /// Colore di sfondo stimato (RGB).
    pub sfondo: [u8; 3],
    /// Colore del testo stimato (RGB).
    pub testo: [u8; 3],
    /// Rapporto di contrasto (1.0–21.0).
    pub rapporto: f64,
    /// Supera AA per testo normale (≥ 4.5:1).
    pub aa_normale: bool,
    /// Supera AA per testo grande/grassetto (≥ 3:1).
    pub aa_grande: bool,
}

/// Luminanza relativa di un colore (formula WCAG).
fn luminanza(c: [u8; 3]) -> f64 {
    let canale = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * canale(c[0]) + 0.7152 * canale(c[1]) + 0.0722 * canale(c[2])
}

/// Rapporto di contrasto WCAG tra due colori (≥ 1.0).
pub fn rapporto(a: [u8; 3], b: [u8; 3]) -> f64 {
    let la = luminanza(a);
    let lb = luminanza(b);
    let (chiaro, scuro) = if la >= lb { (la, lb) } else { (lb, la) };
    (chiaro + 0.05) / (scuro + 0.05)
}

/// Distanza euclidea (al quadrato) tra due colori.
fn distanza2(a: [u8; 3], b: [u8; 3]) -> i32 {
    let d = |x: u8, y: u8| (x as i32 - y as i32).pow(2);
    d(a[0], b[0]) + d(a[1], b[1]) + d(a[2], b[2])
}

/// Stima sfondo/testo di una pagina e ne valuta il contrasto.
pub fn analizza_pagina(percorso: &std::path::Path, pagina: i32) -> Risultato<ContrastoPagina> {
    let img = crate::documento::render_immagine(percorso, pagina, 700)?.to_rgba8();

    // Istogramma su colori quantizzati (bucket da 16 livelli per canale).
    let mut conteggi: HashMap<[u8; 3], u64> = HashMap::new();
    for p in img.pixels() {
        if p[3] < 200 {
            continue; // ignora i pixel molto trasparenti
        }
        let q = [p[0] & 0xF0, p[1] & 0xF0, p[2] & 0xF0];
        *conteggi.entry(q).or_insert(0) += 1;
    }
    if conteggi.is_empty() {
        return Ok(ContrastoPagina {
            pagina,
            sfondo: [255, 255, 255],
            testo: [0, 0, 0],
            rapporto: 21.0,
            aa_normale: true,
            aa_grande: true,
        });
    }

    // Sfondo = colore più frequente.
    let sfondo = *conteggi.iter().max_by_key(|(_, n)| **n).unwrap().0;

    // Testo = colore più frequente abbastanza distante dallo sfondo
    // (evita anti-aliasing e sfumature vicine al fondo).
    let testo = conteggi
        .iter()
        .filter(|(c, _)| distanza2(**c, sfondo) > 6000)
        .max_by_key(|(_, n)| **n)
        .map(|(c, _)| *c)
        // se non c'è testo distinguibile, usa il colore più lontano dallo sfondo
        .unwrap_or_else(|| *conteggi.keys().max_by_key(|c| distanza2(**c, sfondo)).unwrap());

    let r = rapporto(sfondo, testo);
    Ok(ContrastoPagina {
        pagina,
        sfondo,
        testo,
        rapporto: (r * 100.0).round() / 100.0,
        aa_normale: r >= 4.5,
        aa_grande: r >= 3.0,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nero_su_bianco_e_21() {
        let r = rapporto([0, 0, 0], [255, 255, 255]);
        assert!((r - 21.0).abs() < 0.01, "atteso 21, ottenuto {r}");
    }

    #[test]
    fn uguali_danno_1() {
        assert!((rapporto([120, 120, 120], [120, 120, 120]) - 1.0).abs() < 0.001);
    }
}
