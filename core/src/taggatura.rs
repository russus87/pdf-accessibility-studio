//! Generazione di un PDF *realmente* taggato a partire da un documento senza
//! struttura (tipicamente l'output dell'analisi Docling).
//!
//! Strategia: **overlay di testo invisibile taggato** sopra le pagine originali
//! (stessa idea del layer di testo dell'OCR). Per ogni blocco proposto scriviamo
//! il testo in modalita' di rendering invisibile (`3 Tr`), avvolto in una
//! sequenza di marked content `/Ruolo <</MCID n>> BDC … EMC`, e creiamo un
//! `StructElem` che vi punta tramite l'MCID. Cosi' otteniamo, sul PDF originale:
//!   - un albero dei tag (StructTreeRoot → Document → H1/P/Figure…);
//!   - il *marked content* che lega ogni tag al contenuto (cio' che mancava alla
//!     bozza di `correzione::genera_struttura`);
//!   - un ParentTree completo;
//!   - testo estraibile/leggibile dallo screen reader grazie a un font
//!     incorporato (Type0/Identity-H) con ToUnicode.
//!
//! In piu', opzionalmente:
//!   - **embedding del DocLang** come *associated file* (`/AF`), coerente con
//!     PDF/A-3 che nasce proprio per allegare file;
//!   - **metadati PDF/A-3** (XMP `pdfaid`, OutputIntent sRGB, /ID).
//!
//! ATTENZIONE onesta: la piena conformita' PDF/A-3 dipende anche dal *contenuto
//! originale* (font non incorporati, trasparenze… nelle pagine d'origine) e non
//! e' verificabile senza un validatore tipo veraPDF. Trattiamo PDF/A-3 come
//! "best effort": scriviamo le strutture richieste, ma non garantiamo l'esito.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use ttf_parser::{Face, GlyphId};

use crate::errore::{Errore, Risultato};

/// Font incorporato (DejaVu Sans: licenza permissiva, ampia copertura Unicode).
const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
/// Profilo ICC sRGB per l'OutputIntent del PDF/A.
const ICC_SRGB: &[u8] = include_bytes!("../assets/sRGB.icc");

/// Un blocco da taggare (di norma una proposta dell'analisi Docling).
#[derive(Debug, Clone)]
pub struct BloccoTag {
    /// Ruolo PDF/UA standard (P, H1..H6, Figure, Table, L, LI, Caption...).
    pub ruolo: String,
    /// Testo del blocco (vuoto per figure).
    pub testo: String,
    /// Pagina 0-based.
    pub pagina: i32,
    /// Riquadro [l, t, r, b] nelle coordinate di Docling, se disponibile.
    pub bbox: Option<[f32; 4]>,
    /// true se le coordinate hanno origine in basso a sinistra (default Docling),
    /// false se in alto a sinistra.
    pub coord_basso: bool,
    /// Testo alternativo per le figure.
    pub alt: Option<String>,
}

/// Opzioni di generazione.
#[derive(Debug, Clone, Default)]
pub struct OpzioniTag {
    pub lang: Option<String>,
    pub titolo: Option<String>,
    /// Se presente, viene incorporato come associated file DocLang.
    pub doclang: Option<String>,
    /// Se true, aggiunge i metadati PDF/A-3 (XMP, OutputIntent, /ID).
    pub pdfa3: bool,
}

/// Genera il PDF taggato in `destinazione`. Ritorna il numero di elementi creati.
pub fn genera_taggato(
    origine: &Path,
    destinazione: &Path,
    blocchi: &[BloccoTag],
    opt: &OpzioniTag,
) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo (Root) non trovato".into()))?;

    // Pagine in ordine: indice 0-based -> ObjectId.
    let pagine: Vec<(i32, ObjectId)> = doc
        .get_pages()
        .into_iter()
        .map(|(n, id)| (n as i32 - 1, id))
        .collect();
    let page_by_idx: HashMap<i32, ObjectId> = pagine.iter().cloned().collect();
    let mut altezze: HashMap<i32, f32> = HashMap::new();
    for (idx, id) in &pagine {
        altezze.insert(*idx, altezza_pagina(&doc, *id));
    }

    let face = Face::parse(FONT, 0).map_err(|e| Errore::Io(format!("font: {e:?}")))?;
    let upem = face.units_per_em() as f32;
    let scala = 1000.0 / upem;

    // Un id per ogni StructElem-blocco (anche se poi alcuni vengono saltati).
    let elem_ids: Vec<ObjectId> = blocchi.iter().map(|_| doc.new_object_id()).collect();

    // Costruzione del contenuto overlay per pagina + raccolta glifi usati.
    let mut ops: HashMap<i32, String> = HashMap::new();
    let mut mcid_per_pagina: HashMap<i32, i32> = HashMap::new();
    let mut refs_per_pagina: HashMap<i32, Vec<Object>> = HashMap::new();
    let mut mcid_di_blocco: Vec<i32> = vec![0; blocchi.len()];
    let mut validi: Vec<bool> = vec![false; blocchi.len()];
    let mut glifi: BTreeMap<u16, u32> = BTreeMap::new(); // gid -> codepoint (per ToUnicode)

    for (i, b) in blocchi.iter().enumerate() {
        let Some(_) = page_by_idx.get(&b.pagina) else { continue };
        if b.ruolo == "Artifact" {
            continue;
        }
        let mcid = *mcid_per_pagina.get(&b.pagina).unwrap_or(&0);
        mcid_per_pagina.insert(b.pagina, mcid + 1);
        mcid_di_blocco[i] = mcid;
        validi[i] = true;

        let refs = refs_per_pagina.entry(b.pagina).or_default();
        while refs.len() <= mcid as usize {
            refs.push(Object::Null);
        }
        refs[mcid as usize] = Object::Reference(elem_ids[i]);

        let h = *altezze.get(&b.pagina).unwrap_or(&792.0);
        let (x, y) = posizione(b, h, mcid);
        let tag = &b.ruolo;
        let s = ops.entry(b.pagina).or_default();
        if b.testo.trim().is_empty() {
            // Figura o blocco senza testo: marked content vuoto per ancorare il tag.
            s.push_str(&format!("/{tag} <</MCID {mcid}>> BDC EMC\n"));
        } else {
            let hex = glifi_hex(&face, &b.testo, &mut glifi);
            s.push_str(&format!(
                "/{tag} <</MCID {mcid}>> BDC\nBT /F1 10 Tf 3 Tr {x:.2} {y:.2} Td {hex} Tj ET\nEMC\n"
            ));
        }
    }

    // Font incorporato (Type0/Identity-H con ToUnicode). Se non c'e' testo,
    // creiamo comunque il font: pesa poco e semplifica.
    let font_id = costruisci_font(&mut doc, &face, &glifi, scala);

    // Per pagina: appende il content stream, aggiunge il font alle risorse,
    // imposta StructParents e raccoglie le voci del ParentTree.
    let mut parent_nums: Vec<Object> = Vec::new();
    let mut prossima_chiave: i64 = 0;
    for (idx, id) in &pagine {
        let Some(opstr) = ops.get(idx) else { continue };
        let content = format!("q\n{opstr}Q\n");
        let stream_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));
        aggiungi_contenuto(&mut doc, *id, stream_id);
        aggiungi_font(&mut doc, *id, font_id);

        let chiave = prossima_chiave;
        prossima_chiave += 1;
        if let Ok(p) = doc.get_object_mut(*id).and_then(|o| o.as_dict_mut()) {
            p.set("StructParents", Object::Integer(chiave));
        }

        let refs = refs_per_pagina.remove(idx).unwrap_or_default();
        let arr_id = doc.add_object(Object::Array(refs));
        parent_nums.push(Object::Integer(chiave));
        parent_nums.push(Object::Reference(arr_id));
    }

    // --- Albero dei tag ---
    let str_root_id = doc.new_object_id();
    let doc_elem_id = doc.new_object_id();
    let parent_tree_id = doc.new_object_id();

    let mut figli = Vec::new();
    for (i, b) in blocchi.iter().enumerate() {
        if !validi[i] {
            continue;
        }
        let mut d = Dictionary::new();
        d.set("Type", nome("StructElem"));
        d.set("S", nome(&b.ruolo));
        d.set("P", Object::Reference(doc_elem_id));
        d.set("Pg", Object::Reference(page_by_idx[&b.pagina]));
        d.set("K", Object::Integer(mcid_di_blocco[i] as i64));
        if b.ruolo == "Figure" {
            if let Some(alt) = b.alt.as_ref().filter(|a| !a.trim().is_empty()) {
                d.set("Alt", utf16(alt));
            }
        }
        doc.set_object(elem_ids[i], d);
        figli.push(Object::Reference(elem_ids[i]));
    }
    let n_elementi = figli.len();

    let mut docelem = Dictionary::new();
    docelem.set("Type", nome("StructElem"));
    docelem.set("S", nome("Document"));
    docelem.set("P", Object::Reference(str_root_id));
    docelem.set("K", Object::Array(figli));
    doc.set_object(doc_elem_id, docelem);

    let mut pt = Dictionary::new();
    pt.set("Nums", Object::Array(parent_nums));
    doc.set_object(parent_tree_id, pt);

    let mut sr = Dictionary::new();
    sr.set("Type", nome("StructTreeRoot"));
    sr.set("K", Object::Reference(doc_elem_id));
    sr.set("ParentTree", Object::Reference(parent_tree_id));
    sr.set("ParentTreeNextKey", Object::Integer(prossima_chiave));
    doc.set_object(str_root_id, sr);

    // --- Catalogo: struttura, marcatura, lingua, titolo a video ---
    {
        let catalog = doc
            .get_object_mut(catalog_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;
        catalog.set("StructTreeRoot", Object::Reference(str_root_id));
        let mut mi = Dictionary::new();
        mi.set("Marked", Object::Boolean(true));
        catalog.set("MarkInfo", Object::Dictionary(mi));
        if let Some(l) = opt.lang.as_deref().filter(|s| !s.trim().is_empty()) {
            catalog.set("Lang", Object::string_literal(l));
        }
        let mut vp = Dictionary::new();
        vp.set("DisplayDocTitle", Object::Boolean(true));
        catalog.set("ViewerPreferences", Object::Dictionary(vp));
    }

    // Titolo nell'Info.
    if let Some(t) = opt.titolo.as_deref().filter(|s| !s.trim().is_empty()) {
        let info_id = match doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
            Ok(id) => id,
            Err(_) => {
                let id = doc.add_object(Dictionary::new());
                doc.trailer.set("Info", Object::Reference(id));
                id
            }
        };
        if let Ok(info) = doc.get_object_mut(info_id).and_then(|o| o.as_dict_mut()) {
            info.set("Title", utf16(t));
        }
    }

    // --- Embedding DocLang come associated file ---
    if let Some(dl) = opt.doclang.as_deref().filter(|s| !s.trim().is_empty()) {
        incorpora_doclang(&mut doc, catalog_id, dl)?;
    }

    // --- Metadati PDF/A-3 (best effort) ---
    if opt.pdfa3 {
        applica_pdfa3(&mut doc, catalog_id, opt)?;
    }

    doc.save(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(n_elementi)
}

// --- Font incorporato ---------------------------------------------------------

/// Costruisce un font Type0/Identity-H con il TTF incorporato e ritorna l'id del
/// dizionario Type0 (quello referenziato come /F1 nelle risorse).
fn costruisci_font(doc: &mut Document, face: &Face, glifi: &BTreeMap<u16, u32>, scala: f32) -> ObjectId {
    // FontFile2: il programma TrueType grezzo.
    let mut ff_dict = Dictionary::new();
    ff_dict.set("Length1", Object::Integer(FONT.len() as i64));
    let font_file_id = doc.add_object(Stream::new(ff_dict, FONT.to_vec()));

    // FontDescriptor.
    let bb = face.global_bounding_box();
    let mut fd = Dictionary::new();
    fd.set("Type", nome("FontDescriptor"));
    fd.set("FontName", nome("DejaVuSans"));
    fd.set("Flags", Object::Integer(32)); // nonsymbolic
    fd.set(
        "FontBBox",
        Object::Array(vec![
            Object::Integer((bb.x_min as f32 * scala) as i64),
            Object::Integer((bb.y_min as f32 * scala) as i64),
            Object::Integer((bb.x_max as f32 * scala) as i64),
            Object::Integer((bb.y_max as f32 * scala) as i64),
        ]),
    );
    fd.set("ItalicAngle", Object::Integer(0));
    fd.set("Ascent", Object::Integer((face.ascender() as f32 * scala) as i64));
    fd.set("Descent", Object::Integer((face.descender() as f32 * scala) as i64));
    fd.set(
        "CapHeight",
        Object::Integer((face.capital_height().unwrap_or(face.ascender()) as f32 * scala) as i64),
    );
    fd.set("StemV", Object::Integer(80));
    fd.set("FontFile2", Object::Reference(font_file_id));
    let fd_id = doc.add_object(fd);

    // W: larghezze per CID (= GID, perche' CIDToGIDMap = Identity).
    let mut w = Vec::new();
    for (&gid, _) in glifi.iter() {
        let adv = face.glyph_hor_advance(GlyphId(gid)).unwrap_or(0) as f32 * scala;
        w.push(Object::Integer(gid as i64));
        w.push(Object::Array(vec![Object::Integer(adv as i64)]));
    }

    // CIDFontType2.
    let mut cid = Dictionary::new();
    cid.set("Type", nome("Font"));
    cid.set("Subtype", nome("CIDFontType2"));
    cid.set("BaseFont", nome("DejaVuSans"));
    let mut csi = Dictionary::new();
    csi.set("Registry", Object::string_literal("Adobe"));
    csi.set("Ordering", Object::string_literal("Identity"));
    csi.set("Supplement", Object::Integer(0));
    cid.set("CIDSystemInfo", Object::Dictionary(csi));
    cid.set("FontDescriptor", Object::Reference(fd_id));
    cid.set("CIDToGIDMap", nome("Identity"));
    cid.set("DW", Object::Integer(1000));
    if !w.is_empty() {
        cid.set("W", Object::Array(w));
    }
    let cid_id = doc.add_object(cid);

    // ToUnicode CMap.
    let tu_id = doc.add_object(Stream::new(Dictionary::new(), tounicode(glifi)));

    // Type0.
    let mut t0 = Dictionary::new();
    t0.set("Type", nome("Font"));
    t0.set("Subtype", nome("Type0"));
    t0.set("BaseFont", nome("DejaVuSans"));
    t0.set("Encoding", nome("Identity-H"));
    t0.set("DescendantFonts", Object::Array(vec![Object::Reference(cid_id)]));
    t0.set("ToUnicode", Object::Reference(tu_id));
    doc.add_object(t0)
}

/// Stringa esadecimale dei glyph id per il testo (Identity-H = 2 byte per glifo).
fn glifi_hex(face: &Face, testo: &str, glifi: &mut BTreeMap<u16, u32>) -> String {
    let mut s = String::from("<");
    for c in testo.chars() {
        let gid = face.glyph_index(c).map(|g| g.0).unwrap_or(0);
        glifi.entry(gid).or_insert(c as u32);
        s.push_str(&format!("{gid:04X}"));
    }
    s.push('>');
    s
}

/// Costruisce il contenuto della CMap ToUnicode (gid 2-byte -> Unicode UTF-16BE).
fn tounicode(glifi: &BTreeMap<u16, u32>) -> Vec<u8> {
    let mut corpo = String::new();
    let voci: Vec<(u16, u32)> = glifi.iter().map(|(&g, &c)| (g, c)).collect();
    // bfchar a blocchi di max 100.
    for blocco in voci.chunks(100) {
        corpo.push_str(&format!("{} beginbfchar\n", blocco.len()));
        for (gid, cp) in blocco {
            corpo.push_str(&format!("<{gid:04X}> <{}>\n", utf16be_hex(*cp)));
        }
        corpo.push_str("endbfchar\n");
    }
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
/CIDSystemInfo <</Registry (Adobe) /Ordering (UCS) /Supplement 0>> def\n\
/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
{corpo}endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
    )
    .into_bytes()
}

/// Codepoint -> esadecimale UTF-16BE (gestisce i caratteri oltre il BMP).
fn utf16be_hex(cp: u32) -> String {
    let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
    let mut buf = [0u16; 2];
    let mut s = String::new();
    for u in ch.encode_utf16(&mut buf) {
        s.push_str(&format!("{u:04X}"));
    }
    s
}

// --- Posizionamento -----------------------------------------------------------

/// Calcola (x, y) della baseline del testo invisibile. Essendo invisibile, la
/// precisione non e' critica: serve solo un'ancora ragionevole sulla pagina.
fn posizione(b: &BloccoTag, altezza: f32, mcid: i32) -> (f32, f32) {
    match b.bbox {
        Some([l, t, r, bot]) => {
            let _ = r;
            let x = l.max(0.0);
            let y = if b.coord_basso {
                // origine in basso: la baseline sta sul bordo inferiore (min).
                t.min(bot)
            } else {
                // origine in alto: converti il bordo inferiore (max) in coord PDF.
                altezza - t.max(bot)
            };
            (x, y.clamp(2.0, altezza - 2.0))
        }
        // Nessun bbox: impila dall'alto verso il basso.
        None => (40.0, (altezza - 18.0 * (mcid as f32 + 1.0)).max(18.0)),
    }
}

/// Altezza della pagina dal MediaBox (anche ereditato dagli antenati).
fn altezza_pagina(doc: &Document, page_id: ObjectId) -> f32 {
    let mut cur = Some(page_id);
    for _ in 0..16 {
        let Some(id) = cur else { break };
        let Ok(d) = doc.get_object(id).and_then(|o| o.as_dict()) else { break };
        if let Ok(mb) = d.get(b"MediaBox").and_then(|o| o.as_array()) {
            if mb.len() == 4 {
                let y0 = num(&mb[1]);
                let y1 = num(&mb[3]);
                return (y1 - y0).abs().max(1.0);
            }
        }
        cur = d.get(b"Parent").and_then(|o| o.as_reference()).ok();
    }
    792.0
}

fn num(o: &Object) -> f32 {
    match o {
        Object::Integer(i) => *i as f32,
        Object::Real(r) => *r,
        _ => 0.0,
    }
}

// --- Modifiche alle pagine ----------------------------------------------------

/// Appende un content stream alla pagina, preservando quelli esistenti.
fn aggiungi_contenuto(doc: &mut Document, page_id: ObjectId, stream_id: ObjectId) {
    let esistenti = match doc.get_object(page_id).and_then(|o| o.as_dict()) {
        Ok(d) => match d.get(b"Contents") {
            Ok(Object::Reference(r)) => vec![Object::Reference(*r)],
            Ok(Object::Array(a)) => a.clone(),
            _ => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    let mut nuovo = esistenti;
    nuovo.push(Object::Reference(stream_id));
    if let Ok(p) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
        p.set("Contents", Object::Array(nuovo));
    }
}

/// Aggiunge il font /F1 alle risorse della pagina, preservando le risorse
/// (anche ereditate) gia' presenti.
fn aggiungi_font(doc: &mut Document, page_id: ObjectId, font_id: ObjectId) {
    let mut risorse = risorse_effettive(doc, page_id);
    let mut font = match risorse.get(b"Font") {
        Ok(Object::Dictionary(d)) => d.clone(),
        Ok(Object::Reference(r)) => doc
            .get_object(*r)
            .and_then(|o| o.as_dict())
            .cloned()
            .unwrap_or_default(),
        _ => Dictionary::new(),
    };
    font.set("F1", Object::Reference(font_id));
    risorse.set("Font", Object::Dictionary(font));
    if let Ok(p) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
        p.set("Resources", Object::Dictionary(risorse));
    }
}

/// Risorse effettive della pagina (proprie o ereditate dagli antenati), clonate.
fn risorse_effettive(doc: &Document, page_id: ObjectId) -> Dictionary {
    let mut cur = Some(page_id);
    for _ in 0..16 {
        let Some(id) = cur else { break };
        let Ok(d) = doc.get_object(id).and_then(|o| o.as_dict()) else { break };
        match d.get(b"Resources") {
            Ok(Object::Dictionary(r)) => return r.clone(),
            Ok(Object::Reference(r)) => {
                if let Ok(rd) = doc.get_object(*r).and_then(|o| o.as_dict()) {
                    return rd.clone();
                }
            }
            _ => {}
        }
        cur = d.get(b"Parent").and_then(|o| o.as_reference()).ok();
    }
    Dictionary::new()
}

// --- Embedding DocLang --------------------------------------------------------

/// Incorpora il DocLang come associated file (`/AF`) a livello documento.
fn incorpora_doclang(doc: &mut Document, catalog_id: ObjectId, doclang: &str) -> Risultato<()> {
    // Stream del file incorporato.
    let mut ef_dict = Dictionary::new();
    let mut params = Dictionary::new();
    params.set("Size", Object::Integer(doclang.len() as i64));
    ef_dict.set("Type", nome("EmbeddedFile"));
    ef_dict.set("Subtype", nome("text/plain"));
    ef_dict.set("Params", Object::Dictionary(params));
    let ef_id = doc.add_object(Stream::new(ef_dict, doclang.as_bytes().to_vec()));

    // Filespec con relazione "Supplement" (file di supporto al contenuto).
    let mut ef = Dictionary::new();
    ef.set("F", Object::Reference(ef_id));
    ef.set("UF", Object::Reference(ef_id));
    let mut fs = Dictionary::new();
    fs.set("Type", nome("Filespec"));
    fs.set("F", Object::string_literal("struttura.doclang"));
    fs.set("UF", utf16("struttura.doclang"));
    fs.set("EF", Object::Dictionary(ef));
    fs.set("AFRelationship", nome("Supplement"));
    fs.set("Desc", utf16("Struttura semantica DocLang del documento"));
    let fs_id = doc.add_object(fs);

    // Name tree /Names/EmbeddedFiles.
    let mut names_ef = Dictionary::new();
    names_ef.set(
        "Names",
        Object::Array(vec![Object::string_literal("struttura.doclang"), Object::Reference(fs_id)]),
    );
    let names_ef_id = doc.add_object(names_ef);

    let catalog = doc
        .get_object_mut(catalog_id)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;
    let mut names = match catalog.get(b"Names") {
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    names.set("EmbeddedFiles", Object::Reference(names_ef_id));
    catalog.set("Names", Object::Dictionary(names));
    // /AF a livello documento (richiesto da PDF/A-3 per gli associated file).
    catalog.set("AF", Object::Array(vec![Object::Reference(fs_id)]));
    Ok(())
}

// --- PDF/A-3 (best effort) ----------------------------------------------------

/// Aggiunge XMP `pdfaid`, OutputIntent sRGB e /ID. Non garantisce la conformita'
/// completa (dipende anche dal contenuto originale): va validato con veraPDF.
fn applica_pdfa3(doc: &mut Document, catalog_id: ObjectId, opt: &OpzioniTag) -> Risultato<()> {
    // OutputIntent con profilo ICC sRGB.
    let mut icc_dict = Dictionary::new();
    icc_dict.set("N", Object::Integer(3));
    let icc_id = doc.add_object(Stream::new(icc_dict, ICC_SRGB.to_vec()));
    let mut oi = Dictionary::new();
    oi.set("Type", nome("OutputIntent"));
    oi.set("S", nome("GTS_PDFA1"));
    oi.set("OutputConditionIdentifier", Object::string_literal("sRGB IEC61966-2.1"));
    oi.set("Info", Object::string_literal("sRGB IEC61966-2.1"));
    oi.set("DestOutputProfile", Object::Reference(icc_id));
    let oi_id = doc.add_object(oi);

    // XMP con pdfaid part=3, conformance B.
    let titolo = opt.titolo.as_deref().unwrap_or("Documento");
    let lang = opt.lang.as_deref().unwrap_or("x-default");
    let xmp = xmp_pdfa(titolo, lang);
    let mut meta_dict = Dictionary::new();
    meta_dict.set("Type", nome("Metadata"));
    meta_dict.set("Subtype", nome("XML"));
    let meta_id = doc.add_object(Stream::new(meta_dict, xmp.into_bytes()));

    {
        let catalog = doc
            .get_object_mut(catalog_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;
        catalog.set("OutputIntents", Object::Array(vec![Object::Reference(oi_id)]));
        catalog.set("Metadata", Object::Reference(meta_id));
    }

    // /ID nel trailer (richiesto da PDF/A): due stringhe uguali derivate.
    let id_bytes = id_documento();
    let id_obj = Object::String(id_bytes.clone(), StringFormat::Hexadecimal);
    doc.trailer.set(
        "ID",
        Object::Array(vec![id_obj.clone(), id_obj]),
    );
    Ok(())
}

/// Pacchetto XMP minimale per PDF/A-3 (conformance B) con titolo e lingua.
fn xmp_pdfa(titolo: &str, lang: &str) -> String {
    let t = escape_xml(titolo);
    let l = escape_xml(lang);
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
  <rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n\
   <pdfaid:part>3</pdfaid:part>\n   <pdfaid:conformance>B</pdfaid:conformance>\n\
  </rdf:Description>\n\
  <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
   <dc:title><rdf:Alt><rdf:li xml:lang=\"{l}\">{t}</rdf:li></rdf:Alt></dc:title>\n\
   <dc:language><rdf:Bag><rdf:li>{l}</rdf:li></rdf:Bag></dc:language>\n\
  </rdf:Description>\n\
 </rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>"
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Identificativo del documento (16 byte) derivato dal tempo corrente.
fn id_documento() -> Vec<u8> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut v = nanos.to_be_bytes().to_vec(); // 16 byte (u128)
    v.resize(16, 0);
    v
}

// --- Helper comuni ------------------------------------------------------------

fn nome(s: &str) -> Object {
    Object::Name(s.as_bytes().to_vec())
}

/// Stringa testo PDF in UTF-16BE (gestisce gli accenti).
fn utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
