//! Ricerca testuale nel documento (case-insensitive), pagina per pagina.

use std::path::Path;

use serde::Serialize;

use crate::errore::Risultato;

/// Occorrenze in una pagina, con un estratto di contesto.
#[derive(Debug, Clone, Serialize)]
pub struct Occorrenza {
    pub pagina: i32,
    pub conteggio: usize,
    pub contesto: String,
}

/// Cerca `query` in tutte le pagine; ritorna le pagine con almeno un risultato.
pub fn cerca(percorso: &Path, query: &str) -> Risultato<Vec<Occorrenza>> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let pagine = crate::testo_pagine(percorso)?;
    let mut out = Vec::new();
    for (i, testo) in pagine.iter().enumerate() {
        let lower = testo.to_lowercase();
        let conteggio = lower.matches(&q).count();
        if conteggio == 0 {
            continue;
        }
        let pos = lower.find(&q).unwrap_or(0);
        out.push(Occorrenza {
            pagina: i as i32,
            conteggio,
            contesto: contesto(&lower, pos, q.len()),
        });
    }
    Ok(out)
}

/// Estrae un breve contesto attorno alla posizione (in byte) di un match.
fn contesto(testo: &str, byte_pos: usize, len: usize) -> String {
    // Allarga a sinistra/destra di ~40 caratteri, restando ai confini di char.
    let inizio = testo[..byte_pos]
        .char_indices()
        .rev()
        .nth(40)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let fine_match = (byte_pos + len).min(testo.len());
    let fine = testo[fine_match..]
        .char_indices()
        .nth(40)
        .map(|(i, _)| fine_match + i)
        .unwrap_or(testo.len());

    let mut s = testo[inizio..fine].replace(['\n', '\r'], " ").trim().to_string();
    if inizio > 0 {
        s = format!("…{s}");
    }
    if fine < testo.len() {
        s.push('…');
    }
    s
}
