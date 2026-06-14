//! Lettura dei tag (structure tree) di un PDF tramite lopdf.
//!
//! E' la base condivisa di validazione (Fase 2), confronto per tag (Fase 4) ed
//! export dei tag (Fase 5). Estrae: se il documento e' taggato, la lingua, il
//! titolo, e l'albero degli elementi strutturali (ruolo, Alt, ActualText, lang).

use std::collections::{HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;

use crate::errore::{Errore, Risultato};

/// Un nodo dell'albero dei tag (es. un `P`, `H1`, `Figure`, `Table`...).
#[derive(Debug, Clone, Serialize, Default)]
pub struct NodoTag {
    /// Ruolo standard (dopo eventuale rimappatura via RoleMap).
    pub ruolo: String,
    /// Ruolo originale se diverso da quello standard.
    pub ruolo_originale: Option<String>,
    /// Testo alternativo (Alt) — fondamentale per immagini/figure.
    pub alt: Option<String>,
    /// Testo effettivo (ActualText).
    pub actual_text: Option<String>,
    /// Lingua specifica dell'elemento, se presente.
    pub lang: Option<String>,
    /// Indice di pagina (0-based) a cui l'elemento si riferisce, se ricavabile.
    pub pagina: Option<i32>,
    /// Figli nell'albero.
    pub figli: Vec<NodoTag>,
}

/// Informazioni strutturali del documento utili all'accessibilita'.
#[derive(Debug, Clone, Serialize)]
pub struct InfoStruttura {
    /// MarkInfo/Marked = true.
    pub taggato: bool,
    /// Presenza dello StructTreeRoot (albero dei tag).
    pub ha_struct_tree: bool,
    /// Lingua a livello documento (/Lang nel catalogo).
    pub lang: Option<String>,
    /// Titolo dai metadati (Info /Title).
    pub titolo: Option<String>,
    /// ViewerPreferences /DisplayDocTitle (mostra il titolo nella barra).
    pub display_doc_title: Option<bool>,
    /// Elementi radice dell'albero dei tag.
    pub radice: Vec<NodoTag>,
}

/// Analizza un PDF e ne estrae le informazioni strutturali.
pub fn analizza(percorso: &std::path::Path) -> Risultato<InfoStruttura> {
    let doc = Document::load(percorso).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let catalogo = match catalogo(&doc) {
        Some(c) => c,
        None => {
            return Ok(InfoStruttura {
                taggato: false,
                ha_struct_tree: false,
                lang: None,
                titolo: None,
                display_doc_title: None,
                radice: Vec::new(),
            })
        }
    };

    let lang = stringa(&doc, catalogo, b"Lang");
    let titolo = titolo_documento(&doc);
    let display_doc_title = viewer_pref_bool(&doc, catalogo, b"DisplayDocTitle");

    // MarkInfo /Marked
    let taggato = deref(&doc, catalogo.get(b"MarkInfo").ok())
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Marked").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);

    // StructTreeRoot
    let str_root = deref(&doc, catalogo.get(b"StructTreeRoot").ok())
        .and_then(|o| o.as_dict().ok());
    let ha_struct_tree = str_root.is_some();

    // Mappa ObjectId della pagina -> indice 0-based, per risolvere /Pg.
    let mut pagine_idx = HashMap::new();
    for (num, id) in doc.get_pages() {
        pagine_idx.insert(id, num as i32 - 1);
    }

    let mut radice = Vec::new();
    if let Some(root) = str_root {
        let rolemap = rolemap(&doc, root);
        let mut visti = HashSet::new();
        if let Ok(k) = root.get(b"K") {
            raccogli(&doc, k, None, &rolemap, &pagine_idx, None, &mut radice, &mut visti, 0);
        }
    }

    Ok(InfoStruttura {
        taggato,
        ha_struct_tree,
        lang,
        titolo,
        display_doc_title,
        radice,
    })
}

// --- Helper di traversamento -------------------------------------------------

/// Segue le reference fino all'oggetto concreto.
fn deref<'a>(doc: &'a Document, obj: Option<&'a Object>) -> Option<&'a Object> {
    let mut cur = obj?;
    for _ in 0..32 {
        match cur.as_reference() {
            Ok(id) => match doc.get_object(id) {
                Ok(o) => cur = o,
                Err(_) => return None,
            },
            Err(_) => return Some(cur),
        }
    }
    Some(cur)
}

/// Dizionario del catalogo del documento.
fn catalogo(doc: &Document) -> Option<&Dictionary> {
    deref(doc, doc.trailer.get(b"Root").ok())?.as_dict().ok()
}

/// Costruisce la mappa RoleMap (ruolo personalizzato -> ruolo standard).
fn rolemap(doc: &Document, str_root: &Dictionary) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Some(Object::Dictionary(d)) = deref(doc, str_root.get(b"RoleMap").ok()) {
        for (k, v) in d.iter() {
            if let Ok(std) = v.as_name() {
                m.insert(
                    String::from_utf8_lossy(k).into_owned(),
                    String::from_utf8_lossy(std).into_owned(),
                );
            }
        }
    }
    m
}

/// Raccoglie i nodi a partire da un oggetto K (puo' essere array, dict o ref).
#[allow(clippy::too_many_arguments)]
fn raccogli(
    doc: &Document,
    obj: &Object,
    id_riferito: Option<ObjectId>,
    rolemap: &HashMap<String, String>,
    pagine_idx: &HashMap<ObjectId, i32>,
    pagina_ereditata: Option<i32>,
    out: &mut Vec<NodoTag>,
    visti: &mut HashSet<ObjectId>,
    profondita: usize,
) {
    if profondita > 60 {
        return;
    }
    // Evita cicli: se e' una reference gia' vista, fermati.
    let id = obj.as_reference().ok().or(id_riferito);
    if let Some(id) = id {
        if !visti.insert(id) {
            return;
        }
    }

    let Some(risolto) = deref(doc, Some(obj)) else {
        return;
    };

    match risolto {
        Object::Array(arr) => {
            for o in arr {
                raccogli(doc, o, None, rolemap, pagine_idx, pagina_ereditata, out, visti, profondita + 1);
            }
        }
        Object::Dictionary(d) => {
            // E' un elemento strutturale solo se ha la chiave S (il ruolo).
            if let Ok(s) = d.get(b"S").and_then(|o| o.as_name()) {
                let originale = String::from_utf8_lossy(s).into_owned();
                let standard = rolemap.get(&originale).cloned().unwrap_or_else(|| originale.clone());
                // Pagina dell'elemento: da /Pg, altrimenti ereditata dal genitore.
                let pagina = d
                    .get(b"Pg")
                    .ok()
                    .and_then(|o| o.as_reference().ok())
                    .and_then(|id| pagine_idx.get(&id).copied())
                    .or(pagina_ereditata);
                let mut nodo = NodoTag {
                    ruolo: standard.clone(),
                    ruolo_originale: (standard != originale).then_some(originale),
                    alt: stringa(doc, d, b"Alt"),
                    actual_text: stringa(doc, d, b"ActualText"),
                    lang: stringa(doc, d, b"Lang"),
                    pagina,
                    figli: Vec::new(),
                };
                if let Ok(k) = d.get(b"K") {
                    raccogli(doc, k, None, rolemap, pagine_idx, pagina, &mut nodo.figli, visti, profondita + 1);
                }
                out.push(nodo);
            } else if let Ok(k) = d.get(b"K") {
                // Contenitore senza ruolo proprio: scendi comunque.
                raccogli(doc, k, None, rolemap, pagine_idx, pagina_ereditata, out, visti, profondita + 1);
            }
        }
        // Interi (MCID) e altro: contenuto, non struttura.
        _ => {}
    }
}

// --- Helper di lettura valori ------------------------------------------------

/// Legge una stringa testuale da una chiave di dizionario (gestisce UTF-16BE).
fn stringa(doc: &Document, d: &Dictionary, chiave: &[u8]) -> Option<String> {
    match deref(doc, d.get(chiave).ok())? {
        Object::String(b, _) => Some(decodifica(b)),
        Object::Name(b) => Some(String::from_utf8_lossy(b).into_owned()),
        _ => None,
    }
}

/// Titolo dal dizionario Info del trailer.
fn titolo_documento(doc: &Document) -> Option<String> {
    let info = deref(doc, doc.trailer.get(b"Info").ok())?.as_dict().ok()?;
    stringa(doc, info, b"Title").filter(|s| !s.trim().is_empty())
}

/// Legge un booleano dalle ViewerPreferences del catalogo.
fn viewer_pref_bool(doc: &Document, catalogo: &Dictionary, chiave: &[u8]) -> Option<bool> {
    let vp = deref(doc, catalogo.get(b"ViewerPreferences").ok())?
        .as_dict()
        .ok()?;
    vp.get(chiave).ok()?.as_bool().ok()
}

/// Decodifica una stringa PDF: UTF-16BE con BOM oppure PDFDocEncoding (~Latin1).
fn decodifica(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16s: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|c| ((c[0] as u16) << 8) | (*c.get(1).unwrap_or(&0) as u16))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}
