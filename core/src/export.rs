//! Export dell'albero dei tag in JSON o XML (Fase 5).

use std::path::Path;

use crate::errore::{Errore, Risultato};
use crate::struttura::{analizza, NodoTag};

/// Esporta l'intera struttura (info documento + albero tag) in JSON indentato.
pub fn esporta_json(percorso: &Path) -> Risultato<String> {
    let info = analizza(percorso)?;
    serde_json::to_string_pretty(&info).map_err(|e| Errore::Io(e.to_string()))
}

/// Esporta l'albero dei tag in XML.
pub fn esporta_xml(percorso: &Path) -> Risultato<String> {
    let info = analizza(percorso)?;
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<StructTree");
    if let Some(l) = &info.lang {
        s.push_str(&format!(" lang=\"{}\"", esc(l)));
    }
    if let Some(t) = &info.titolo {
        s.push_str(&format!(" title=\"{}\"", esc(t)));
    }
    s.push_str(&format!(" tagged=\"{}\">\n", info.taggato));
    for n in &info.radice {
        scrivi_nodo(&mut s, n, 1);
    }
    s.push_str("</StructTree>\n");
    Ok(s)
}

fn scrivi_nodo(s: &mut String, n: &NodoTag, prof: usize) {
    let ind = "  ".repeat(prof);
    s.push_str(&ind);
    s.push_str(&format!("<Tag role=\"{}\"", esc(&n.ruolo)));
    if let Some(r) = &n.ruolo_originale {
        s.push_str(&format!(" roleOriginale=\"{}\"", esc(r)));
    }
    if let Some(a) = &n.alt {
        s.push_str(&format!(" alt=\"{}\"", esc(a)));
    }
    if let Some(a) = &n.actual_text {
        s.push_str(&format!(" actualText=\"{}\"", esc(a)));
    }
    if let Some(l) = &n.lang {
        s.push_str(&format!(" lang=\"{}\"", esc(l)));
    }
    if n.figli.is_empty() {
        s.push_str("/>\n");
    } else {
        s.push_str(">\n");
        for f in &n.figli {
            scrivi_nodo(s, f, prof + 1);
        }
        s.push_str(&ind);
        s.push_str("</Tag>\n");
    }
}

/// Escape minimale per attributi/testo XML.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
