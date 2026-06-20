//! Ponte verso il sidecar Docling per l'analisi della struttura dei PDF *non
//! taggati* (auto-tagging assistito).
//!
//! Stesso schema di `piper.rs`: una dipendenza esterna che non e' incorporata
//! nel binario. Qui pero' non scarichiamo un engine: Docling e' una libreria
//! Python che l'utente installa una volta (`pip install docling`). Noi ci
//! limitiamo a:
//!   1. depositare lo script-ponte nella cartella dati dell'app (`docling_prepara`);
//!   2. rilevare se Python e Docling sono disponibili (`docling_stato`);
//!   3. lanciare lo script su un PDF e tradurne il JSON in proposte di tag
//!      (`analizza_struttura_ai`), **solo se il PDF non e' gia' taggato**.
//!
//! La logica di parsing/mappatura vive nel core (`pdfa_core::doclang`), questo
//! modulo si occupa solo del processo esterno.

use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::stato::StatoApp;

/// Lo script-ponte, incorporato nel binario e scritto su disco al bisogno: cosi'
/// resta allineato alla versione dell'app senza problemi di packaging.
const BRIDGE_PY: &str = include_str!("../sidecar/docling_bridge.py");

/// Cartella dati dell'app per i file di Docling.
fn dir_docling(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("docling"))
        .map_err(|e| format!("cartella dati non disponibile: {e}"))
}

/// Percorso in cui depositiamo lo script-ponte.
fn percorso_script(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dir_docling(app)?.join("docling_bridge.py"))
}

/// Verifica che un interprete sia eseguibile (`--version` va a buon fine).
fn eseguibile(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Trova un interprete Python qualsiasi nel PATH (per dire "Python c'e'").
fn python_bin() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|b| eseguibile(b))
}

/// Risolve il percorso assoluto di un eseguibile nel PATH (cross-platform).
fn risolvi_eseguibile(nome: &str) -> Option<PathBuf> {
    let (shell, flag) = if cfg!(windows) {
        ("where", nome)
    } else {
        ("sh", nome)
    };
    let output = if cfg!(windows) {
        Command::new(shell).arg(flag).output().ok()?
    } else {
        Command::new(shell)
            .arg("-c")
            .arg(format!("command -v {nome}"))
            .output()
            .ok()?
    };
    if !output.status.success() {
        return None;
    }
    let primo = String::from_utf8_lossy(&output.stdout);
    let primo = primo.lines().next()?.trim();
    (!primo.is_empty()).then(|| PathBuf::from(primo))
}

/// Se `script` e' un console-script con shebang `#!/.../python`, ne ricava
/// l'interprete. E' il caso di pipx/venv: il `docling` nel PATH punta cosi' al
/// Python del proprio ambiente isolato (su Windows i wrapper sono .exe: None).
fn python_da_shebang(script: &std::path::Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    let f = std::fs::File::open(script).ok()?;
    let mut riga = String::new();
    BufReader::new(f).read_line(&mut riga).ok()?;
    let riga = riga.trim();
    let resto = riga.strip_prefix("#!")?.trim();
    // Accetta solo shebang che puntano a un interprete python.
    (resto.contains("python") && !resto.is_empty()).then(|| resto.to_string())
}

/// Interpreti Python candidati, in ordine di priorita':
///   1. override esplicito via `DOCLING_PYTHON`;
///   2. il Python del venv di `docling` (ricavato dal lanciatore nel PATH:
///      copre pipx e i venv, dove il `python` di sistema NON vede il pacchetto);
///   3. `python3` / `python` di sistema (pip "classico").
fn interpreti_candidati() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("DOCLING_PYTHON") {
        if !p.trim().is_empty() {
            v.push(p);
        }
    }
    if let Some(script) = risolvi_eseguibile("docling") {
        if let Some(py) = python_da_shebang(&script) {
            v.push(py);
        }
    }
    v.push("python3".to_string());
    v.push("python".to_string());
    v
}

/// Verifica se `docling` e' importabile da `bin` (senza importarlo davvero:
/// `find_spec` non carica i modelli, quindi resta veloce).
fn docling_importabile(bin: &str) -> bool {
    Command::new(bin)
        .args([
            "-c",
            "import importlib.util,sys; sys.exit(0 if importlib.util.find_spec('docling') else 1)",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Ritorna il primo interprete candidato che esegue e vede `docling`.
fn python_con_docling() -> Option<String> {
    interpreti_candidati()
        .into_iter()
        .find(|bin| eseguibile(bin) && docling_importabile(bin))
}

/// Stato del ponte Docling per la UI.
#[derive(Serialize)]
pub struct StatoDocling {
    /// Sistema operativo ("windows" | "macos" | "linux" | ...): la UI ci mostra
    /// le istruzioni di installazione giuste.
    pub piattaforma: String,
    /// Un interprete Python e' disponibile nel PATH.
    pub python_disponibile: bool,
    /// Quale interprete e' stato trovato (es. "python3").
    pub python_bin: Option<String>,
    /// Il pacchetto `docling` e' installato in quell'interprete.
    pub docling_installato: bool,
    /// Lo script-ponte e' stato depositato nella cartella dati.
    pub script_pronto: bool,
}

/// Ritorna lo stato del ponte: Python, Docling, script.
#[tauri::command]
pub fn docling_stato(app: AppHandle) -> StatoDocling {
    // Interprete che vede davvero docling (gestisce pipx/venv); se nessuno,
    // ripieghiamo sul python di sistema solo per dire se Python esiste.
    let con_docling = python_con_docling();
    let py_sistema = python_bin();
    let script_pronto = percorso_script(&app).map(|p| p.exists()).unwrap_or(false);
    StatoDocling {
        piattaforma: std::env::consts::OS.to_string(),
        python_disponibile: con_docling.is_some() || py_sistema.is_some(),
        python_bin: con_docling.clone().or_else(|| py_sistema.map(|b| b.to_string())),
        docling_installato: con_docling.is_some(),
        script_pronto,
    }
}

/// Deposita (o aggiorna) lo script-ponte nella cartella dati dell'app.
#[tauri::command]
pub fn docling_prepara(app: AppHandle) -> Result<(), String> {
    let dest = percorso_script(&app)?;
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&dest, BRIDGE_PY).map_err(|e| format!("scrittura script: {e}"))
}

/// Si assicura che lo script esista (lo scrive se manca) e ne ritorna il percorso.
fn assicura_script(app: &AppHandle) -> Result<PathBuf, String> {
    let p = percorso_script(app)?;
    if !p.exists() {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, BRIDGE_PY).map_err(|e| format!("scrittura script: {e}"))?;
    }
    Ok(p)
}

/// Analizza la struttura di un PDF **non taggato** con Docling e ritorna le
/// proposte di tag PDF/UA.
///
/// Si rifiuta di lavorare su un PDF che ha gia' uno StructTreeRoot: l'analisi AI
/// e' pensata solo per i documenti senza taggatura, come richiesto.
///
/// E' un comando **asincrono**: Docling puo' impiegare minuti (al primo avvio
/// scarica i modelli), quindi il lavoro pesante gira su un thread dedicato
/// (`spawn_blocking`) e il thread principale resta libero — la UI non si frizza.
#[tauri::command]
pub async fn analizza_struttura_ai(
    app: AppHandle,
    id: String,
    stato: State<'_, StatoApp>,
) -> Result<pdfa_core::AnalisiDoclang, String> {
    // Estraiamo il percorso prima di cedere il thread: il MutexGuard non e' Send
    // e non va tenuto oltre l'await.
    let path = {
        stato.documenti.lock().unwrap().get(&id).cloned()
    }
    .ok_or_else(|| "documento non aperto".to_string())?;

    // Lavoro bloccante (subprocess Docling) fuori dal thread principale.
    tauri::async_runtime::spawn_blocking(move || esegui_analisi(&app, &path))
        .await
        .map_err(|e| format!("analisi interrotta: {e}"))?
}

/// Corpo sincrono dell'analisi: validazione, rilevamento Python, lancio del
/// ponte Docling e parsing del JSON. Pensato per girare in `spawn_blocking`.
fn esegui_analisi(
    app: &AppHandle,
    path: &std::path::Path,
) -> Result<pdfa_core::AnalisiDoclang, String> {
    // Vincolo esplicito: solo PDF privi di struttura.
    let info = pdfa_core::analizza(path).map_err(|e| e.to_string())?;
    if info.ha_struct_tree {
        return Err(
            "Il PDF e' gia' taggato: l'analisi AI serve solo ai documenti senza struttura.".into(),
        );
    }

    let bin = python_con_docling().ok_or_else(|| {
        if python_bin().is_some() {
            "Docling non installato: esegui 'pipx install docling' (oppure 'pip install docling')."
                .to_string()
        } else {
            "Python non trovato nel PATH. Installa Python 3 e poi 'pipx install docling'."
                .to_string()
        }
    })?;

    let script = assicura_script(app)?;
    let output = Command::new(&bin)
        .arg(&script)
        .arg(path)
        .output()
        .map_err(|e| format!("avvio del ponte Docling: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let msg = err.trim();
        return Err(if msg.is_empty() {
            "il ponte Docling ha restituito un errore".into()
        } else {
            format!("Docling: {msg}")
        });
    }

    let json = String::from_utf8_lossy(&output.stdout);
    pdfa_core::analizza_doclang(&json).map_err(|e| e.to_string())
}

/// Una proposta di tag inviata dalla UI per generare il PDF taggato.
#[derive(serde::Deserialize)]
pub struct PropostaInput {
    pub ruolo: String,
    #[serde(default)]
    pub pagina: Option<i32>,
    #[serde(default)]
    pub testo: String,
    #[serde(default)]
    pub bbox: Option<[f32; 4]>,
    #[serde(default)]
    pub coord_basso: bool,
}

/// Genera un PDF **realmente taggato** dalle proposte: overlay di testo
/// invisibile taggato con marked content sopra le pagine originali, font
/// incorporato e albero dei tag completo. Opzionalmente incorpora il DocLang e
/// scrive i metadati PDF/A-3. Ritorna il numero di elementi creati.
///
/// NB: il PDF/A-3 e' best effort (dipende anche dal contenuto originale) e va
/// validato con uno strumento esterno tipo veraPDF.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn genera_pdf_accessibile(
    id: String,
    proposte: Vec<PropostaInput>,
    lang: Option<String>,
    titolo: Option<String>,
    doclang: Option<String>,
    pdfa3: bool,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<usize, String> {
    let path = stato
        .documenti
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "documento non aperto".to_string())?;

    // Gli artifact (intestazioni/pie' di pagina) non vanno nell'albero dei tag.
    let blocchi: Vec<pdfa_core::taggatura::BloccoTag> = proposte
        .into_iter()
        .filter(|p| p.ruolo != "Artifact")
        .filter_map(|p| {
            p.pagina.map(|pagina| pdfa_core::taggatura::BloccoTag {
                // Per le figure usiamo il testo come Alt se disponibile.
                alt: (p.ruolo == "Figure" && !p.testo.trim().is_empty()).then(|| p.testo.clone()),
                ruolo: p.ruolo,
                testo: p.testo,
                pagina,
                bbox: p.bbox,
                coord_basso: p.coord_basso,
            })
        })
        .collect();

    if blocchi.is_empty() {
        return Err("nessun elemento da taggare".into());
    }

    let opzioni = pdfa_core::taggatura::OpzioniTag {
        lang: lang.filter(|s| !s.trim().is_empty()),
        titolo: titolo.filter(|s| !s.trim().is_empty()),
        doclang: doclang.filter(|s| !s.trim().is_empty()),
        pdfa3,
    };

    pdfa_core::taggatura::genera_taggato(
        &path,
        std::path::Path::new(&destinazione),
        &blocchi,
        &opzioni,
    )
    .map_err(|e| e.to_string())
}
