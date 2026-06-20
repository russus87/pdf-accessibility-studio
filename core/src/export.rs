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

/// Esporta l'albero dei tag in **DocTags** (la serializzazione testuale di
/// Docling/DocLang). A differenza dell'export Docling vero e proprio — che gira
/// solo sui PDF *non* taggati — qui partiamo dallo StructTree gia' presente nel
/// documento: la struttura semantica e l'ordine di lettura sono fedeli, ma non
/// ci sono i token di posizione (`<loc_*>`), perche' il tag tree non porta le
/// coordinate di layout.
pub fn esporta_doclang(percorso: &Path) -> Risultato<String> {
    let info = analizza(percorso)?;
    if !info.ha_struct_tree {
        return Err(Errore::Io(
            "il PDF non ha un albero dei tag: niente struttura da esportare in DocLang".into(),
        ));
    }
    Ok(serializza_doctags(&info.radice))
}

/// Serializza un albero di `NodoTag` in DocTags. Funzione pura (testabile da
/// sola): il documento e' racchiuso in `<doctag>`, ogni elemento usa l'etichetta
/// Docling corrispondente al suo ruolo PDF/UA.
pub fn serializza_doctags(radice: &[NodoTag]) -> String {
    let mut s = String::from("<doctag>\n");
    for n in radice {
        scrivi_doctag(&mut s, n, 1);
    }
    s.push_str("</doctag>\n");
    s
}

fn scrivi_doctag(s: &mut String, n: &NodoTag, prof: usize) {
    // Il nodo Document e' solo il contenitore radice: lo rendiamo trasparente,
    // <doctag> fa gia' da involucro.
    if n.ruolo == "Document" {
        for f in &n.figli {
            scrivi_doctag(s, f, prof);
        }
        return;
    }
    let tag = etichetta_doctags(&n.ruolo);
    let ind = "  ".repeat(prof);
    // Contenuto testuale: l'ActualText o, per le figure, l'Alt come descrizione.
    let testo = n
        .actual_text
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .or_else(|| n.alt.as_deref().filter(|t| !t.trim().is_empty()));
    let ha_figli = !n.figli.is_empty();

    s.push_str(&ind);
    s.push('<');
    s.push_str(&tag);
    s.push('>');
    if let Some(t) = testo {
        s.push_str(&esc(t));
    }
    if ha_figli {
        s.push('\n');
        for f in &n.figli {
            scrivi_doctag(s, f, prof + 1);
        }
        s.push_str(&ind);
    }
    s.push_str("</");
    s.push_str(&tag);
    s.push_str(">\n");
}

/// Mappa un ruolo PDF/UA sull'etichetta DocTags piu' vicina. I ruoli ignoti
/// diventano il loro nome in minuscolo: l'output resta ben formato senza
/// perdere informazione (es. `Span` -> `span`, `Link` -> `link`).
fn etichetta_doctags(ruolo: &str) -> String {
    match ruolo {
        "H1" => "section_header_level_1",
        "H2" => "section_header_level_2",
        "H3" => "section_header_level_3",
        "H4" => "section_header_level_4",
        "H5" => "section_header_level_5",
        "H6" => "section_header_level_6",
        "Title" => "title",
        "P" => "text",
        "L" => "unordered_list",
        "LI" => "list_item",
        "Table" => "otsl",
        "TR" => "table_row",
        "TH" | "TD" => "table_cell",
        "Figure" => "picture",
        "Caption" => "caption",
        "Note" => "footnote",
        "Code" => "code",
        "Formula" => "formula",
        altro => return altro.to_ascii_lowercase(),
    }
    .to_string()
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

#[cfg(test)]
mod test {
    use super::*;

    fn nodo(ruolo: &str, testo: Option<&str>, figli: Vec<NodoTag>) -> NodoTag {
        NodoTag {
            ruolo: ruolo.to_string(),
            actual_text: testo.map(|t| t.to_string()),
            figli,
            ..Default::default()
        }
    }

    #[test]
    fn doctags_struttura_e_ruoli() {
        // Document trasparente, titolo, paragrafo, lista con item, figura via Alt.
        let mut figura = nodo("Figure", None, vec![]);
        figura.alt = Some("Grafico delle vendite".into());
        let albero = vec![nodo(
            "Document",
            None,
            vec![
                nodo("H1", Some("Relazione"), vec![]),
                nodo("P", Some("Testo & <prova>"), vec![]),
                nodo("L", None, vec![nodo("LI", Some("Primo"), vec![])]),
                figura,
                nodo("Span", Some("inline"), vec![]), // ruolo ignoto -> minuscolo
            ],
        )];

        let out = serializza_doctags(&albero);

        // Involucro radice e Document trasparente (niente <document>).
        assert!(out.starts_with("<doctag>\n"));
        assert!(out.trim_end().ends_with("</doctag>"));
        assert!(!out.contains("<document>"));
        // Mappatura ruoli.
        assert!(out.contains("<section_header_level_1>Relazione</section_header_level_1>"));
        assert!(out.contains("<unordered_list>"));
        assert!(out.contains("<list_item>Primo</list_item>"));
        // L'Alt della figura diventa il contenuto della <picture>.
        assert!(out.contains("<picture>Grafico delle vendite</picture>"));
        // Ruolo ignoto -> nome in minuscolo.
        assert!(out.contains("<span>inline</span>"));
        // Escape XML nel testo.
        assert!(out.contains("Testo &amp; &lt;prova&gt;"));
    }
}
