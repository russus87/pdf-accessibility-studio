//! OCR dei PDF scansionati tramite il programma esterno `tesseract`.
//!
//! Renderizza ogni pagina in PNG e lascia a tesseract il lavoro di riconoscere
//! il testo e produrre un PDF con livello testo ricercabile (e quindi accessibile
//! agli screen reader). tesseract va installato dall'utente (+ i dati lingua).

use std::path::Path;
use std::process::Command;

use crate::errore::{Errore, Risultato};

/// Verifica se `tesseract` è disponibile nel sistema.
pub fn disponibile() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Elenco delle lingue installate in tesseract (es. ["eng", "ita"]).
pub fn lingue() -> Vec<String> {
    let Ok(out) = Command::new("tesseract").arg("--list-langs").output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .skip(1) // la prima riga è un'intestazione
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.contains(' '))
        .collect()
}

/// Esegue l'OCR su tutte le pagine e salva un PDF ricercabile in `destinazione`.
/// `lingua` è un codice tesseract (es. "ita", "eng", "ita+eng").
pub fn ocr_a_pdf(origine: &Path, destinazione: &Path, lingua: &str) -> Risultato<()> {
    if !disponibile() {
        return Err(Errore::Io("tesseract non è installato".into()));
    }

    let info = crate::apri(origine)?;
    if info.pagine == 0 {
        return Err(Errore::Io("documento senza pagine".into()));
    }

    // Cartella temporanea dedicata.
    let dir = std::env::temp_dir().join(format!("pdfa_ocr_{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;

    // Renderizza ogni pagina (~200 DPI su A4) e annota i percorsi.
    let mut elenco = String::new();
    for p in 0..info.pagine as i32 {
        let png = crate::render_pagina(origine, p, 1654)?;
        let file = dir.join(format!("pag_{p:04}.png"));
        std::fs::write(&file, png)?;
        elenco.push_str(&file.to_string_lossy());
        elenco.push('\n');
    }
    let lista = dir.join("pagine.txt");
    std::fs::write(&lista, elenco)?;

    // tesseract <lista> <base> pdf -l <lingua>  ->  <base>.pdf
    let base = dir.join("ocr_out");
    let stato = Command::new("tesseract")
        .arg(&lista)
        .arg(&base)
        .arg("-l")
        .arg(lingua)
        .arg("pdf")
        .output()
        .map_err(|e| Errore::Io(format!("avvio tesseract: {e}")))?;

    if !stato.status.success() {
        let err = String::from_utf8_lossy(&stato.stderr);
        let _ = std::fs::remove_dir_all(&dir);
        return Err(Errore::Io(format!("tesseract: {}", err.trim())));
    }

    let prodotto = dir.join("ocr_out.pdf");
    std::fs::copy(&prodotto, destinazione)?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
