//! Editor di accessibilità delle tabelle (Fase 19).
//!
//! Rende le tabelle realmente accessibili scrivendo, sulle celle (StructElem
//! `TH`/`TD`), l'attributo di layout tabellare `/A <</O /Table …>>`:
//!
//! - **Scope** sulle intestazioni `TH` (`/Scope /Row|/Column|/Both`): dice agli
//!   screen reader se l'intestazione vale per la riga, la colonna o entrambe;
//! - **RowSpan / ColSpan** per le celle unite.
//!
//! Le modifiche vengono fuse con un eventuale attributo Table già presente e
//! salvate in una copia (l'originale resta intatto). L'associazione esplicita
//! TD→TH via Headers/ID arriverà come rifinitura successiva.

use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::errore::{Errore, Risultato};

/// Attributi richiesti per una cella di tabella. I campi `Option` non impostati
/// lasciano invariato il valore esistente; `scope: Some("")` rimuove lo Scope.
#[derive(Debug, Clone, Default)]
pub struct AttributiCella {
    /// Riferimento "numero_generazione" della cella (TH/TD).
    pub riferimento: String,
    /// "Row" | "Column" | "Both" | "" (rimuovi). `None` = non toccare.
    pub scope: Option<String>,
    /// Numero di righe occupate (>=1). `None` = non toccare; `Some(1)` rimuove.
    pub row_span: Option<i64>,
    /// Numero di colonne occupate (>=1). `None` = non toccare; `Some(1)` rimuove.
    pub col_span: Option<i64>,
}

/// Applica gli attributi alle celle indicate e salva una copia in `destinazione`.
/// Ritorna quante celle sono state modificate.
pub fn applica(origine: &Path, destinazione: &Path, celle: &[AttributiCella]) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    // Prima passata (in sola lettura): per ogni cella calcola il nuovo dizionario
    // di attributi Table, partendo da quello esistente.
    let mut piano: Vec<(ObjectId, Dictionary)> = Vec::new();
    for c in celle {
        let Some(id) = parse_riferimento(&c.riferimento) else {
            continue;
        };
        let Ok(cell) = doc.get_object(id).and_then(|o| o.as_dict()) else {
            continue;
        };
        let mut attr = attr_tabella_esistente(&doc, cell);

        if let Some(sc) = &c.scope {
            match sc.trim() {
                "" => {
                    attr.remove(b"Scope");
                }
                "Row" | "Column" | "Both" => attr.set("Scope", Object::Name(sc.as_bytes().to_vec())),
                _ => {}
            }
        }
        applica_span(&mut attr, b"RowSpan", c.row_span);
        applica_span(&mut attr, b"ColSpan", c.col_span);

        piano.push((id, attr));
    }

    // Seconda passata: scrive l'attributo /A sulle celle.
    let mut count = 0;
    for (id, attr) in piano {
        if let Ok(cell) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
            cell.set("A", Object::Dictionary(attr));
            count += 1;
        }
    }

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(count)
}

/// Imposta o rimuove uno span: `Some(n>1)` lo scrive, `Some(1)` lo rimuove
/// (valore di default), `None` non tocca nulla.
fn applica_span(attr: &mut Dictionary, chiave: &[u8], valore: Option<i64>) {
    match valore {
        Some(n) if n > 1 => attr.set(chiave, Object::Integer(n)),
        Some(_) => {
            attr.remove(chiave);
        }
        None => {}
    }
}

/// Ritorna il dizionario degli attributi Table già presente sulla cella (clonato),
/// oppure un nuovo dizionario con `/O /Table`.
fn attr_tabella_esistente(doc: &Document, cell: &Dictionary) -> Dictionary {
    if let Some(a) = deref(doc, cell.get(b"A").ok()) {
        match a {
            Object::Dictionary(d) if is_table(d) => return d.clone(),
            Object::Array(arr) => {
                for o in arr {
                    if let Some(Object::Dictionary(d)) = deref(doc, Some(o)) {
                        if is_table(d) {
                            return d.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let mut d = Dictionary::new();
    d.set("O", Object::Name(b"Table".to_vec()));
    d
}

/// Vero se il dizionario di attributi ha `/O /Table`.
fn is_table(d: &Dictionary) -> bool {
    d.get(b"O").and_then(|o| o.as_name()).map(|n| n == b"Table").unwrap_or(false)
}

/// Segue le reference fino all'oggetto concreto.
fn deref<'a>(doc: &'a Document, obj: Option<&'a Object>) -> Option<&'a Object> {
    let mut cur = obj?;
    for _ in 0..32 {
        match cur.as_reference() {
            Ok(id) => cur = doc.get_object(id).ok()?,
            Err(_) => return Some(cur),
        }
    }
    Some(cur)
}

/// Converte un riferimento "numero_generazione" in un ObjectId.
fn parse_riferimento(s: &str) -> Option<ObjectId> {
    let (n, g) = s.split_once('_')?;
    Some((n.parse().ok()?, g.parse().ok()?))
}
