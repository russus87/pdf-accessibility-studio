//! Sommario (TOC) automatico (Fase 41): genera una pagina indice dai titoli
//! (Hn) del documento, con i numeri di pagina, e la antepone al PDF.

use std::path::Path;

use crate::errore::{Errore, Risultato};

fn livello(ruolo: &str) -> Option<u8> {
    match ruolo {
        "Title" | "H1" => Some(1),
        _ => ruolo.strip_prefix('H').and_then(|s| s.parse::<u8>().ok()).filter(|n| (1..=6).contains(n)),
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Genera un sommario dai titoli e lo antepone, salvando una copia.
/// Ritorna il numero di voci del sommario.
pub fn genera(origine: &Path, destinazione: &Path) -> Risultato<usize> {
    let blocchi = crate::lettura::blocchi(origine)?;
    let titoli: Vec<(u8, String, i32)> = blocchi
        .iter()
        .filter_map(|b| livello(&b.ruolo).map(|l| (l, b.testo.clone(), b.pagina.unwrap_or(0))))
        .filter(|(_, t, _)| !t.trim().is_empty())
        .collect();
    if titoli.is_empty() {
        return Err(Errore::Pdfium("nessun titolo (H1..H6) trovato per il sommario".into()));
    }

    // HTML del sommario.
    let mut righe = String::new();
    for (lvl, testo, pag) in &titoli {
        let indent = (*lvl as i32 - 1).max(0) * 16;
        righe.push_str(&format!(
            "<div class=\"v\" style=\"margin-left:{indent}px\"><span class=\"t\">{}</span><span class=\"p\">{}</span></div>\n",
            escape(testo), pag + 1
        ));
    }
    let html = format!(
        "<!doctype html><html lang=\"it\"><head><meta charset=\"utf-8\"><style>\
         body{{font-family:sans-serif;margin:24mm;color:#111}}\
         h1{{font-size:20pt;margin-bottom:10mm}}\
         .v{{display:flex;justify-content:space-between;border-bottom:1px dotted #bbb;padding:3px 0;font-size:12pt}}\
         .p{{color:#555;margin-left:8px}}</style></head><body><h1>Sommario</h1>{righe}</body></html>"
    );

    let pdf_toc = crate::modello::html_a_pdf(&html)?;
    let tmp = std::env::temp_dir().join(format!("pdfa_toc_{}.pdf", std::process::id()));
    std::fs::write(&tmp, pdf_toc).map_err(|e| Errore::Io(format!("toc temp: {e}")))?;

    crate::pagine::inserisci(origine, &tmp, 1, destinazione)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(titoli.len())
}
