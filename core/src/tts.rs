//! Sintesi vocale di riserva tramite espeak-ng (o espeak), usata quando il
//! webview non ha voci di sistema (tipico su Linux senza speech-dispatcher).
//!
//! Sintetizza il testo in un WAV (PCM) che la UI riproduce con un <audio>.
//! Fase A del lettore vocale; una voce neurale (Piper) potra' arrivare dopo.

use std::io::Write;
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Nome del binario espeak disponibile nel sistema, se presente.
fn binario() -> Option<&'static str> {
    for b in ["espeak-ng", "espeak"] {
        let ok = Command::new(b)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some(b);
        }
    }
    None
}

/// Indica se un motore espeak e' utilizzabile.
pub fn disponibile() -> bool {
    binario().is_some()
}

/// Una voce/lingua offerta da espeak.
#[derive(Debug, Clone, Serialize)]
pub struct VoceTts {
    /// Codice lingua (es. "it", "en-gb").
    pub codice: String,
    /// Nome leggibile (es. "Italian").
    pub nome: String,
}

/// Elenca le lingue disponibili in espeak (vuoto se non installato).
pub fn voci() -> Vec<VoceTts> {
    let Some(bin) = binario() else {
        return Vec::new();
    };
    let Ok(out) = Command::new(bin).arg("--voices").output() else {
        return Vec::new();
    };
    let testo = String::from_utf8_lossy(&out.stdout);
    let mut viste = std::collections::HashSet::new();
    let mut voci = Vec::new();
    for (i, riga) in testo.lines().enumerate() {
        if i == 0 {
            continue; // intestazione
        }
        let parti: Vec<&str> = riga.split_whitespace().collect();
        if parti.len() < 4 {
            continue;
        }
        let codice = parti[1].to_string();
        if !viste.insert(codice.clone()) {
            continue;
        }
        voci.push(VoceTts {
            nome: parti[3].to_string(),
            codice,
        });
    }
    voci.sort_by(|a, b| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()));
    voci
}

/// Sintetizza `testo` nella lingua data e a una velocita' relativa (1.0 = normale),
/// restituendo i byte di un file WAV.
pub fn sintetizza(testo: &str, lingua: &str, velocita: f32) -> Risultato<Vec<u8>> {
    let bin = binario().ok_or_else(|| Errore::Io("espeak-ng non installato".into()))?;
    let lingua = if lingua.trim().is_empty() { "it" } else { lingua.trim() };
    let wpm = (175.0 * velocita).round().clamp(80.0, 450.0) as i32;

    let mut child = Command::new(bin)
        .arg("-v")
        .arg(lingua)
        .arg("-s")
        .arg(wpm.to_string())
        .arg("--stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Errore::Io(format!("avvio espeak: {e}")))?;

    child
        .stdin
        .take()
        .ok_or_else(|| Errore::Io("stdin espeak non disponibile".into()))?
        .write_all(testo.as_bytes())
        .map_err(|e| Errore::Io(format!("scrittura testo: {e}")))?;

    let out = child
        .wait_with_output()
        .map_err(|e| Errore::Io(format!("attesa espeak: {e}")))?;
    if !out.status.success() {
        return Err(Errore::Io("espeak ha restituito un errore".into()));
    }
    Ok(out.stdout)
}
