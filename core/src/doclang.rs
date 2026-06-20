//! Ponte verso Docling / DocLang per i PDF *senza* taggatura.
//!
//! Docling (libreria IBM, Python) sa *inferire* con modelli ML la struttura
//! semantica di un PDF che non ha uno StructTreeRoot: titoli, paragrafi, liste,
//! tabelle, figure, con la posizione sulla pagina. Lo `struttura.rs` nativo, al
//! contrario, sa solo *leggere* i tag gia' presenti. Questo modulo copre quindi
//! il buco: prende il JSON normalizzato prodotto dal bridge Python
//! (`src-tauri/sidecar/docling_bridge.py`) e lo trasforma in **proposte di tag**
//! PDF/UA, pronte per il flusso di `correzione.rs`.
//!
//! Niente dipendenze da Tauri: e' logica pura, testabile da sola. Il lancio del
//! processo Python vive nel layer Tauri (`src-tauri/src/docling.rs`), come gia'
//! avviene per Piper.
//!
//! NB: DocLang (lo standard aperto LF AI & Data, 2026) e' qui solo come PoC:
//! conserviamo la sua serializzazione testuale se il bridge riesce a produrla,
//! ma le proposte di tag derivano dalla struttura normalizzata, piu' stabile.

use serde::{Deserialize, Serialize};

use crate::errore::{Errore, Risultato};
use crate::struttura::NodoTag;

/// Struttura semantica normalizzata cosi' come arriva dal bridge Python.
/// I campi sono volutamente tolleranti: il bridge gira su una libreria di terze
/// parti in rapida evoluzione, quindi tutto ha un default.
#[derive(Debug, Clone, Deserialize)]
pub struct StrutturaDocling {
    /// Lingua principale rilevata, se disponibile (es. "it").
    #[serde(default)]
    pub lingua: Option<String>,
    /// Blocchi di contenuto nell'ordine di lettura inferito da Docling.
    #[serde(default)]
    pub blocchi: Vec<BloccoDocling>,
    /// PoC: serializzazione DocLang/DocTags del documento, se prodotta.
    #[serde(default)]
    pub doclang: Option<String>,
    /// Nome del metodo di export usato per `doclang` (es. "export_to_doctags").
    #[serde(default)]
    pub doclang_fmt: Option<String>,
}

/// Un singolo blocco inferito (un titolo, un paragrafo, una figura...).
#[derive(Debug, Clone, Deserialize)]
pub struct BloccoDocling {
    /// Etichetta semantica grezza di Docling (es. "section_header", "text",
    /// "list_item", "table", "picture", "caption", "title").
    #[serde(default)]
    pub label: String,
    /// Testo del blocco (vuoto per figure/tabelle senza testo proprio).
    #[serde(default)]
    pub testo: String,
    /// Numero di pagina 1-based come lo riporta Docling (None se assente).
    #[serde(default)]
    pub pagina: Option<i32>,
    /// Riquadro [left, top, right, bottom] in coordinate pagina, se disponibile.
    #[serde(default)]
    pub bbox: Option<[Option<f32>; 4]>,
    /// Origine delle coordinate del bbox ("BOTTOMLEFT" | "TOPLEFT"), se nota.
    #[serde(default)]
    pub coord: Option<String>,
    /// Livello gerarchico per i titoli (1 = piu' alto), se Docling lo fornisce.
    #[serde(default)]
    pub livello: Option<i32>,
}

/// Una proposta di tag PDF/UA derivata da un blocco Docling.
#[derive(Debug, Clone, Serialize)]
pub struct PropostaTag {
    /// Ruolo PDF/UA suggerito (P, H1..H6, L, LI, Table, Figure, Caption...).
    pub ruolo: String,
    /// Etichetta Docling di origine (per trasparenza nella UI).
    pub label_origine: String,
    /// Anteprima testuale (troncata) del contenuto.
    pub testo: String,
    /// Pagina 0-based (coerente col resto del core), se ricavabile.
    pub pagina: Option<i32>,
    /// Se la figura/immagine avra' bisogno di un Alt (manca testo descrittivo).
    pub richiede_alt: bool,
    /// Riquadro [l, t, r, b] in coordinate pagina, se tutti e 4 i valori ci sono.
    pub bbox: Option<[f32; 4]>,
    /// true se le coordinate del bbox hanno origine in basso a sinistra.
    pub coord_basso: bool,
}

/// Esito completo dell'analisi assistita, pronto per la UI.
#[derive(Debug, Clone, Serialize)]
pub struct Analisi {
    /// Lingua suggerita per il documento.
    pub lingua: Option<String>,
    /// Numero di blocchi analizzati.
    pub blocchi_totali: usize,
    /// Proposte di tag in ordine di lettura.
    pub proposte: Vec<PropostaTag>,
    /// Le stesse proposte come albero `NodoTag`, cosi' la UI dei tag esistente
    /// puo' mostrarle senza un componente nuovo.
    pub albero: Vec<NodoTag>,
    /// PoC DocLang: serializzazione semantica, se disponibile.
    pub doclang: Option<String>,
    /// Formato della serializzazione `doclang`.
    pub doclang_fmt: Option<String>,
}

/// Analizza il JSON normalizzato del bridge e produce le proposte di tag.
pub fn analizza_json(json: &str) -> Risultato<Analisi> {
    let s: StrutturaDocling =
        serde_json::from_str(json).map_err(|e| Errore::Io(format!("JSON Docling non valido: {e}")))?;
    Ok(analizza_struttura(&s))
}

/// Variante che lavora su una struttura gia' deserializzata (comoda nei test).
pub fn analizza_struttura(s: &StrutturaDocling) -> Analisi {
    let proposte: Vec<PropostaTag> = s.blocchi.iter().map(proposta_da_blocco).collect();
    let albero = proposte.iter().map(nodo_da_proposta).collect();
    Analisi {
        lingua: s.lingua.clone().filter(|l| !l.trim().is_empty()),
        blocchi_totali: s.blocchi.len(),
        proposte,
        albero,
        doclang: s.doclang.clone().filter(|d| !d.trim().is_empty()),
        doclang_fmt: s.doclang_fmt.clone(),
    }
}

/// Costruisce una proposta di tag da un blocco Docling.
fn proposta_da_blocco(b: &BloccoDocling) -> PropostaTag {
    let ruolo = mappa_ruolo(&b.label, b.livello);
    let e_figura = ruolo == "Figure";
    // Testo completo: serve integro al layer di testo taggato (l'anteprima nella
    // UI viene clippata via CSS).
    let testo = b.testo.trim().to_string();
    // Il bbox e' utile solo se tutti e 4 i valori sono presenti.
    let bbox = b.bbox.and_then(|[l, t, r, bot]| Some([l?, t?, r?, bot?]));
    PropostaTag {
        // Le figure senza testo richiedono un Alt: lo segnaliamo.
        richiede_alt: e_figura && testo.is_empty(),
        ruolo,
        label_origine: b.label.clone(),
        testo,
        // Docling numera le pagine da 1; il resto del core usa 0-based.
        pagina: b.pagina.map(|p| (p - 1).max(0)),
        bbox,
        // Docling usa di norma l'origine in basso a sinistra.
        coord_basso: b
            .coord
            .as_deref()
            .map(|c| c.eq_ignore_ascii_case("BOTTOMLEFT"))
            .unwrap_or(true),
    }
}

/// Trasforma una proposta in un `NodoTag` (piatto) riutilizzabile dalla UI.
fn nodo_da_proposta(p: &PropostaTag) -> NodoTag {
    NodoTag {
        ruolo: p.ruolo.clone(),
        ruolo_originale: Some(p.label_origine.clone()),
        // Per le figure il testo non e' contenuto reale: lasciamo Alt vuoto
        // (da compilare), altrimenti mettiamo il testo come ActualText.
        alt: None,
        actual_text: (!p.testo.is_empty()).then(|| p.testo.clone()),
        lang: None,
        pagina: p.pagina,
        riferimento: None,
        figli: Vec::new(),
        ..Default::default()
    }
}

/// Mappa l'etichetta semantica di Docling sul ruolo standard PDF/UA piu' vicino.
/// Per i titoli usa il livello (clamp 1..=6) quando disponibile.
pub fn mappa_ruolo(label: &str, livello: Option<i32>) -> String {
    let l = label.trim().to_ascii_lowercase();
    // Normalizza prefissi tipo "DocItemLabel.SECTION_HEADER".
    let l = l.rsplit('.').next().unwrap_or(&l);
    match l {
        "title" | "document_title" => "H1".to_string(),
        "section_header" | "subtitle_level_1" | "heading" => {
            let n = livello.unwrap_or(2).clamp(1, 6);
            format!("H{n}")
        }
        "paragraph" | "text" | "page_header_text" => "P".to_string(),
        "list" => "L".to_string(),
        "list_item" => "LI".to_string(),
        "table" => "Table".to_string(),
        "picture" | "figure" | "image" => "Figure".to_string(),
        "caption" => "Caption".to_string(),
        "footnote" => "Note".to_string(),
        "code" => "Code".to_string(),
        "formula" | "equation" => "Formula".to_string(),
        // Indice / sommario: tipo di struttura PDF/UA dedicato.
        "document_index" | "table_of_contents" | "toc" => "TOC".to_string(),
        // Voce bibliografica.
        "reference" | "bibliography" => "BibEntry".to_string(),
        // Campi modulo (anche checkbox): tipo di struttura Form.
        "form" | "checkbox_selected" | "checkbox_unselected" => "Form".to_string(),
        // Header/footer: NON sono struttura, vanno trattati come artifact (il
        // generatore li esclude dall'albero dei tag).
        "page_header" | "page_footer" => "Artifact".to_string(),
        // Sconosciuto: ripieghiamo su paragrafo, ruolo neutro e sempre valido.
        _ => "P".to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mappa_ruoli_principali() {
        assert_eq!(mappa_ruolo("title", None), "H1");
        assert_eq!(mappa_ruolo("section_header", Some(3)), "H3");
        assert_eq!(mappa_ruolo("section_header", None), "H2");
        assert_eq!(mappa_ruolo("text", None), "P");
        assert_eq!(mappa_ruolo("list_item", None), "LI");
        assert_eq!(mappa_ruolo("picture", None), "Figure");
        assert_eq!(mappa_ruolo("DocItemLabel.CAPTION", None), "Caption");
        assert_eq!(mappa_ruolo("qualcosa_di_ignoto", None), "P");
    }

    #[test]
    fn mappa_ruoli_estesi() {
        // Indice, bibliografia e moduli: ruoli PDF/UA dedicati (non piu' "P").
        assert_eq!(mappa_ruolo("document_index", None), "TOC");
        assert_eq!(mappa_ruolo("DocItemLabel.REFERENCE", None), "BibEntry");
        assert_eq!(mappa_ruolo("form", None), "Form");
        assert_eq!(mappa_ruolo("checkbox_selected", None), "Form");
        assert_eq!(mappa_ruolo("code", None), "Code");
        assert_eq!(mappa_ruolo("formula", None), "Formula");
        // Header/footer restano "Artifact" (esclusi dall'albero dal generatore).
        assert_eq!(mappa_ruolo("page_footer", None), "Artifact");
    }

    #[test]
    fn livello_titolo_in_range() {
        assert_eq!(mappa_ruolo("section_header", Some(99)), "H6");
        assert_eq!(mappa_ruolo("section_header", Some(0)), "H1");
    }

    #[test]
    fn analizza_json_completo() {
        let j = r#"{
            "lingua": "it",
            "blocchi": [
                {"label":"title","testo":"Relazione annuale","pagina":1,"livello":null},
                {"label":"section_header","testo":"Introduzione","pagina":1,"livello":1},
                {"label":"text","testo":"Testo del paragrafo.","pagina":1},
                {"label":"picture","testo":"","pagina":2}
            ],
            "doclang": "<doc>...</doc>",
            "doclang_fmt": "export_to_doctags"
        }"#;
        let a = analizza_json(j).unwrap();
        assert_eq!(a.lingua.as_deref(), Some("it"));
        assert_eq!(a.blocchi_totali, 4);
        assert_eq!(a.proposte[0].ruolo, "H1");
        assert_eq!(a.proposte[1].ruolo, "H1"); // section_header livello 1
        assert_eq!(a.proposte[2].ruolo, "P");
        assert_eq!(a.proposte[3].ruolo, "Figure");
        // La figura senza testo deve richiedere un Alt.
        assert!(a.proposte[3].richiede_alt);
        assert!(!a.proposte[2].richiede_alt);
        // Pagina convertita a 0-based.
        assert_eq!(a.proposte[0].pagina, Some(0));
        assert_eq!(a.proposte[3].pagina, Some(1));
        // Albero riusabile popolato.
        assert_eq!(a.albero.len(), 4);
        assert_eq!(a.albero[2].actual_text.as_deref(), Some("Testo del paragrafo."));
        assert_eq!(a.doclang_fmt.as_deref(), Some("export_to_doctags"));
    }

    #[test]
    fn json_malformato_e_errore() {
        assert!(analizza_json("non-json").is_err());
    }
}
