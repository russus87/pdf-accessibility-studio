//! Estrazione del contenuto in testo / HTML / Markdown (Fase 32).
//!
//! Usa l'ordine logico dei tag (`lettura::blocchi`) per ricostruire la struttura
//! (titoli, paragrafi, liste); se il PDF non è taggato ripiega sul testo per pagina.

use std::path::Path;

use crate::errore::Risultato;

/// Formato di esportazione.
#[derive(Debug, Clone, Copy)]
pub enum Formato {
    Testo,
    Html,
    Markdown,
}

impl Formato {
    pub fn da_str(s: &str) -> Formato {
        match s {
            "html" => Formato::Html,
            "md" | "markdown" => Formato::Markdown,
            _ => Formato::Testo,
        }
    }
}

/// Estrae il contenuto del PDF nel formato richiesto.
pub fn esporta(origine: &Path, formato: Formato) -> Risultato<String> {
    let blocchi = crate::lettura::blocchi(origine).unwrap_or_default();

    // Fallback: nessun tag -> testo per pagina.
    if blocchi.is_empty() {
        let pagine = crate::testo_pagine(origine)?;
        let testo = pagine.join("\n\n");
        return Ok(match formato {
            Formato::Html => format!(
                "<!doctype html>\n<html><head><meta charset=\"utf-8\"></head>\n<body>\n{}\n</body></html>",
                pagine.iter().map(|p| format!("<p>{}</p>", escape_html(p))).collect::<Vec<_>>().join("\n")
            ),
            _ => testo,
        });
    }

    Ok(match formato {
        Formato::Testo => blocchi.iter().map(|b| b.testo.clone()).collect::<Vec<_>>().join("\n\n"),
        Formato::Html => costruisci_html(&blocchi),
        Formato::Markdown => costruisci_md(&blocchi),
    })
}

fn livello_heading(ruolo: &str) -> Option<u8> {
    match ruolo {
        "Title" | "H1" => Some(1),
        _ => ruolo.strip_prefix('H').and_then(|s| s.parse::<u8>().ok()).filter(|n| (1..=6).contains(n)),
    }
}

fn costruisci_html(blocchi: &[crate::lettura::BloccoLettura]) -> String {
    let mut out = String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\"></head>\n<body>\n");
    let mut in_lista = false;
    for b in blocchi {
        let t = escape_html(&b.testo);
        let e_li = b.ruolo == "LI" || b.ruolo == "LBody";
        if in_lista && !e_li {
            out.push_str("</ul>\n");
            in_lista = false;
        }
        if let Some(n) = livello_heading(&b.ruolo) {
            out.push_str(&format!("<h{n}>{t}</h{n}>\n"));
        } else if e_li {
            if !in_lista {
                out.push_str("<ul>\n");
                in_lista = true;
            }
            out.push_str(&format!("<li>{t}</li>\n"));
        } else if b.ruolo == "Caption" {
            out.push_str(&format!("<p><em>{t}</em></p>\n"));
        } else {
            out.push_str(&format!("<p>{t}</p>\n"));
        }
    }
    if in_lista {
        out.push_str("</ul>\n");
    }
    out.push_str("</body></html>");
    out
}

fn costruisci_md(blocchi: &[crate::lettura::BloccoLettura]) -> String {
    let mut out = String::new();
    for b in blocchi {
        let t = b.testo.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(n) = livello_heading(&b.ruolo) {
            out.push_str(&format!("{} {t}\n\n", "#".repeat(n as usize)));
        } else if b.ruolo == "LI" || b.ruolo == "LBody" {
            out.push_str(&format!("- {t}\n"));
        } else if b.ruolo == "Caption" {
            out.push_str(&format!("*{t}*\n\n"));
        } else {
            out.push_str(&format!("{t}\n\n"));
        }
    }
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
