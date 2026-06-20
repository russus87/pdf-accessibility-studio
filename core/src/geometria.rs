//! Geometria per l'evidenziazione nel visore: rettangoli (normalizzati 0..1 con
//! origine in alto a sinistra) delle occorrenze di ricerca e degli elementi
//! taggati.
//!
//! Pdfium usa coordinate in punti con origine in basso a sinistra; qui le
//! convertiamo in frazioni della pagina con origine in alto a sinistra, cosi'
//! la UI puo' disegnare i riquadri sopra l'immagine renderizzata a qualsiasi
//! zoom senza conoscere le dimensioni in pixel.

use std::path::Path;

use pdfium_render::prelude::*;
use serde::Serialize;

use crate::documento::con_documento;
use crate::errore::Risultato;

/// Rettangolo normalizzato (0..1), origine in alto a sinistra.
#[derive(Debug, Clone, Serialize)]
pub struct Rett {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// Riquadri di un elemento taggato su una pagina (per evidenziarlo nel visore).
#[derive(Debug, Clone, Serialize)]
pub struct RiquadroTag {
    /// Pagina (0-based) dell'elemento.
    pub pagina: i32,
    /// Riquadri trovati (vuoto se non localizzabile: la UI scrolla comunque).
    pub rettangoli: Vec<Rett>,
}

/// Converte un `PdfRect` (origine basso-sx) in `Rett` normalizzato (origine alto-sx).
fn norm(r: PdfRect, pw: f32, ph: f32) -> Rett {
    let pw = if pw <= 0.0 { 1.0 } else { pw };
    let ph = if ph <= 0.0 { 1.0 } else { ph };
    Rett {
        x0: (r.left().value / pw).clamp(0.0, 1.0),
        x1: (r.right().value / pw).clamp(0.0, 1.0),
        y0: ((ph - r.top().value) / ph).clamp(0.0, 1.0),
        y1: ((ph - r.bottom().value) / ph).clamp(0.0, 1.0),
    }
}

/// Rettangoli (normalizzati) delle occorrenze di `query` in una pagina. Se
/// `solo_prima` e' true ritorna solo i riquadri della prima occorrenza (utile
/// per localizzare un singolo elemento), altrimenti tutte (ricerca testuale).
fn rettangoli(percorso: &Path, pagina: i32, query: &str, solo_prima: bool) -> Risultato<Vec<Rett>> {
    let q = query.trim().to_string();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    con_documento(percorso, |doc| {
        let page = doc.pages().get(pagina)?;
        let pw = page.width().value;
        let ph = page.height().value;
        let text = page.text()?;
        let opzioni = PdfSearchOptions::new();
        let mut out = Vec::new();
        if let Ok(search) = text.search(&q, &opzioni) {
            for segments in search.iter(PdfSearchDirection::SearchForward) {
                for seg in segments.iter() {
                    out.push(norm(seg.bounds(), pw, ph));
                }
                if solo_prima {
                    break;
                }
            }
        }
        Ok(out)
    })
}

/// Rettangoli di tutte le occorrenze di `query` in una pagina (evidenziazione ricerca).
pub fn rettangoli_ricerca(percorso: &Path, pagina: i32, query: &str) -> Risultato<Vec<Rett>> {
    rettangoli(percorso, pagina, query, false)
}

/// Riquadro (best-effort) di un elemento taggato, identificato dal suo
/// riferimento ("numero_generazione"). Localizza l'elemento cercandone il testo
/// nella pagina: funziona per paragrafi, titoli e tabelle con testo; per figure
/// senza testo ritorna solo la pagina (la UI scrolla senza riquadro).
pub fn riquadro_tag(percorso: &Path, riferimento: &str) -> Risultato<Option<RiquadroTag>> {
    let Some((pagina, testo)) = crate::lettura::testo_elemento(percorso, riferimento)? else {
        return Ok(None);
    };
    let Some(pagina) = pagina else {
        return Ok(Some(RiquadroTag { pagina: 0, rettangoli: Vec::new() }));
    };

    // Termini di ricerca via parole iniziali, in ordine di "lunghezza" decrescente:
    // piu' parole = match piu' specifico; se non trova, ripiega su meno parole.
    let parole: Vec<&str> = testo.split_whitespace().collect();
    let rettangoli = if parole.is_empty() {
        Vec::new()
    } else {
        let mut trovati = Vec::new();
        for n in [10usize, 6, 4, 2, 1] {
            let n = n.min(parole.len());
            let termine = parole[..n].join(" ");
            trovati = rettangoli(percorso, pagina, &termine, true)?;
            if !trovati.is_empty() {
                break;
            }
        }
        trovati
    };

    Ok(Some(RiquadroTag { pagina, rettangoli }))
}
