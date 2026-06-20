//! Marcatura di elementi come **Artifact** (Fase 18).
//!
//! Gli artifact (intestazioni, piè di pagina, numeri di pagina, decori) NON
//! devono far parte dell'ordine di lettura: gli screen reader li ignorano. In
//! PDF cio' significa due cose, che questo modulo applica entrambe salvando una
//! copia (l'originale resta intatto):
//!
//! 1. l'elemento (StructElem) viene staccato dall'albero dei tag — rimosso dal
//!    `/K` del suo genitore, con tutto il suo sotto-albero;
//! 2. il suo *marked content* nei content stream viene riscritto da taggato
//!    (`/Ruolo <</MCID n>> BDC … EMC`) ad artifact (`/Artifact BDC … EMC`),
//!    eliminando l'MCID.
//!
//! Nota: le voci corrispondenti nel ParentTree restano (orfane, innocue): non
//! essendoci piu' ne' l'MCID nel contenuto ne' l'elemento nell'albero, non
//! influenzano l'ordine di lettura.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::errore::{Errore, Risultato};

/// Marca come Artifact gli elementi indicati (per riferimento "numero_generazione")
/// e salva una copia in `destinazione`. Ritorna quanti elementi sono stati marcati.
pub fn marca_artifact(origine: &Path, destinazione: &Path, riferimenti: &[String]) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let targets: Vec<ObjectId> = riferimenti.iter().filter_map(|s| parse_riferimento(s)).collect();
    if targets.is_empty() {
        return Ok(0);
    }

    // 1. Raccoglie, per ogni pagina (ObjectId), gli MCID che diventano artifact:
    //    quelli dell'elemento e di tutto il suo sotto-albero.
    let mut da_artifact: HashMap<ObjectId, HashSet<i64>> = HashMap::new();
    for &t in &targets {
        let mut visti = HashSet::new();
        raccogli_mcid(&doc, t, None, &mut da_artifact, &mut visti, 0);
    }

    // 2. Riscrive il marked content delle pagine coinvolte.
    let pagine: Vec<(ObjectId, HashSet<i64>)> = da_artifact.into_iter().collect();
    for (page_id, mcids) in pagine {
        riscrivi_content(&mut doc, page_id, &mcids)?;
    }

    // 3. Stacca gli elementi dall'albero (rimuove i riferimenti dai /K dei genitori).
    let target_set: HashSet<ObjectId> = targets.iter().copied().collect();
    rimuovi_dai_genitori(&mut doc, &target_set);

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(targets.len())
}

/// Raccoglie ricorsivamente gli (pagina, MCID) sotto un StructElem.
fn raccogli_mcid(
    doc: &Document,
    id: ObjectId,
    pagina_ered: Option<ObjectId>,
    out: &mut HashMap<ObjectId, HashSet<i64>>,
    visti: &mut HashSet<ObjectId>,
    prof: usize,
) {
    if prof > 80 || !visti.insert(id) {
        return;
    }
    let Ok(d) = doc.get_object(id).and_then(|o| o.as_dict()) else {
        return;
    };
    let pagina = d
        .get(b"Pg")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .or(pagina_ered);
    if let Ok(k) = d.get(b"K") {
        scorri_k(doc, k, pagina, out, visti, prof);
    }
}

/// Scorre il `/K` accumulando gli MCID e ricorrendo nei figli-elemento.
fn scorri_k(
    doc: &Document,
    obj: &Object,
    pagina: Option<ObjectId>,
    out: &mut HashMap<ObjectId, HashSet<i64>>,
    visti: &mut HashSet<ObjectId>,
    prof: usize,
) {
    if prof > 80 {
        return;
    }
    // Riferimento: figlio-elemento (ha /S) o, raramente, un MCR indiretto.
    if let Ok(id) = obj.as_reference() {
        if let Ok(d) = doc.get_object(id).and_then(|o| o.as_dict()) {
            if d.get(b"S").is_ok() {
                raccogli_mcid(doc, id, pagina, out, visti, prof + 1);
            } else {
                mcr(d, pagina, out);
            }
        }
        return;
    }
    match obj {
        Object::Array(arr) => {
            for o in arr {
                scorri_k(doc, o, pagina, out, visti, prof + 1);
            }
        }
        // MCID diretto del contenuto sulla pagina corrente.
        Object::Integer(mcid) => {
            if let Some(p) = pagina {
                out.entry(p).or_default().insert(*mcid);
            }
        }
        Object::Dictionary(d) => {
            if d.get(b"S").is_ok() {
                // Elemento figlio inline (senza id proprio).
                let p = d.get(b"Pg").ok().and_then(|o| o.as_reference().ok()).or(pagina);
                if let Ok(k) = d.get(b"K") {
                    scorri_k(doc, k, p, out, visti, prof + 1);
                }
            } else {
                mcr(d, pagina, out);
            }
        }
        _ => {}
    }
}

/// Estrae l'MCID da un dizionario MCR (marked content reference).
fn mcr(d: &Dictionary, pagina: Option<ObjectId>, out: &mut HashMap<ObjectId, HashSet<i64>>) {
    if let Ok(mcid) = d.get(b"MCID").and_then(|o| o.as_i64()) {
        let p = d.get(b"Pg").ok().and_then(|o| o.as_reference().ok()).or(pagina);
        if let Some(p) = p {
            out.entry(p).or_default().insert(mcid);
        }
    }
}

/// Riscrive il content stream di una pagina cambiando in `/Artifact` i BDC i cui
/// MCID sono nell'insieme, e sostituendo il /Contents con un unico stream nuovo.
fn riscrivi_content(doc: &mut Document, page_id: ObjectId, mcids: &HashSet<i64>) -> Risultato<()> {
    let dati = match doc.get_page_content(page_id) {
        Ok(d) => d,
        Err(_) => return Ok(()), // pagina senza contenuto: niente da fare
    };
    let mut content = Content::decode(&dati).map_err(|e| Errore::Pdfium(format!("content: {e}")))?;

    let mut modificato = false;
    for op in &mut content.operations {
        if op.operator == "BDC" {
            if let Some(mcid) = mcid_da_operandi(&op.operands) {
                if mcids.contains(&mcid) {
                    // Artifact con dizionario di proprieta' vuoto (niente MCID).
                    op.operands = vec![Object::Name(b"Artifact".to_vec()), Object::Dictionary(Dictionary::new())];
                    modificato = true;
                }
            }
        }
    }
    if !modificato {
        return Ok(());
    }

    let nuovo = content.encode().map_err(|e| Errore::Pdfium(format!("encode: {e}")))?;
    let new_id = doc.add_object(Stream::new(Dictionary::new(), nuovo));
    let page = doc
        .get_object_mut(page_id)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| Errore::Pdfium(format!("pagina: {e}")))?;
    page.set("Contents", Object::Reference(new_id));
    Ok(())
}

/// Rimuove dai `/K` di tutti gli elementi (e dello StructTreeRoot) i riferimenti
/// agli elementi target, staccandoli (con i loro sotto-alberi) dall'albero.
fn rimuovi_dai_genitori(doc: &mut Document, targets: &HashSet<ObjectId>) {
    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in ids {
        if targets.contains(&id) {
            continue;
        }
        let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) else {
            continue;
        };
        let Ok(k) = d.get(b"K") else { continue };
        if let Some(nuovo) = k_senza_target(k.clone(), targets) {
            d.set("K", nuovo);
        }
    }
}

/// Ritorna un nuovo `/K` senza i riferimenti ai target, o `None` se invariato.
fn k_senza_target(k: Object, targets: &HashSet<ObjectId>) -> Option<Object> {
    match k {
        Object::Reference(id) if targets.contains(&id) => Some(Object::Array(Vec::new())),
        Object::Array(arr) => {
            let prima = arr.len();
            let filtrato: Vec<Object> = arr
                .into_iter()
                .filter(|o| !matches!(o.as_reference(), Ok(id) if targets.contains(&id)))
                .collect();
            (filtrato.len() != prima).then_some(Object::Array(filtrato))
        }
        _ => None,
    }
}

/// Estrae l'MCID dagli operandi di un BDC (proprieta' inline con /MCID).
fn mcid_da_operandi(operandi: &[Object]) -> Option<i64> {
    if let Some(Object::Dictionary(d)) = operandi.get(1) {
        return d.get(b"MCID").ok()?.as_i64().ok();
    }
    None
}

/// Converte un riferimento "numero_generazione" in un ObjectId.
fn parse_riferimento(s: &str) -> Option<ObjectId> {
    let (n, g) = s.split_once('_')?;
    Some((n.parse().ok()?, g.parse().ok()?))
}
