<script>
  // Lettore vocale stile screen reader. Legge il testo nell'ordine delle pagine
  // e, dove il PDF e' taggato, intercala il testo alternativo delle immagini.
  // Evidenzia la parola in lettura (eventi onboundary) e fa seguire il visore.
  import { schede } from "../lib/schede.svelte.js";
  import {
    testoDocumento, alberoTag, blocchiLettura,
    ttsInfo, ttsSintesi,
    piperStato, piperScaricaEngine, piperScaricaVoce, piperSintesi,
  } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  const sintesi = typeof window !== "undefined" ? window.speechSynthesis : null;
  // Elemento audio per i motori backend (espeak / piper, che producono WAV).
  const audio = typeof Audio !== "undefined" ? new Audio() : null;

  // Motore di lettura: "sistema" (voci del webview), "espeak" o "piper" (backend).
  let motore = $state(null);
  let motoreUtente = $state(false); // true se scelto esplicitamente dall'utente
  let espeakDisponibile = $state(false);
  let espeakVoci = $state([]);
  // Piper (voce neurale, scaricata a runtime).
  let piper = $state({ supportato: false, engine_pronto: false, voci: [] });
  let piperAzione = $state(null); // messaggio durante i download
  const piperVociInstallate = $derived(piper.voci.filter((v) => v.installata));

  // Ogni blocco: { testo, tipo: "testo"|"immagine", pagina }.
  let blocchi = $state([]);
  let indice = $state(0);
  let parolaIndice = $state(0); // offset carattere della parola corrente
  let inLettura = $state(false);
  let inPausa = $state(false);
  let caricamento = $state(false);
  let errore = $state(null);

  let voci = $state([]);
  let voceScelta = $state("");
  let velocita = $state(1);
  let seguiPagina = $state(true);
  let ordineLogico = $state(false); // true se i blocchi vengono dai tag (MCID)

  function inFrasi(testo) {
    return (testo.match(/[^.!?\n]+[.!?]*/g) || []).map((t) => t.trim()).filter(Boolean);
  }

  // Motori attualmente utilizzabili, in ordine di preferenza per l'auto-scelta.
  function motoriDisponibili() {
    const out = [];
    if (voci.length) out.push("sistema");
    if (espeakDisponibile) out.push("espeak");
    if (piper.engine_pronto && piperVociInstallate.length) out.push("piper");
    return out;
  }

  // Voce predefinita per un motore (preferendo l'italiano).
  function defaultVoce(m) {
    if (m === "sistema") {
      const it = voci.find((v) => v.lang.toLowerCase().startsWith("it"));
      return (it || voci[0])?.name || "";
    }
    if (m === "espeak") {
      const it = espeakVoci.find((v) => v.codice.toLowerCase().startsWith("it"));
      return (it || espeakVoci[0] || { codice: "it" }).codice;
    }
    if (m === "piper") {
      const it = piperVociInstallate.find((v) => v.lingua === "it");
      return (it || piperVociInstallate[0])?.id || "";
    }
    return "";
  }

  // Ricalcola i motori disponibili; sceglie un default solo se l'utente non ha deciso.
  function aggiornaMotore() {
    voci = sintesi ? sintesi.getVoices() : [];
    const disp = motoriDisponibili();
    if (motoreUtente && disp.includes(motore)) {
      if (!voceScelta) voceScelta = defaultVoce(motore);
      return;
    }
    const scelto = disp[0] || null;
    motore = scelto;
    voceScelta = scelto ? defaultVoce(scelto) : "";
  }

  // Cambio di motore esplicito dall'utente.
  function cambiaMotore(m) {
    motoreUtente = true;
    fermaInterno();
    motore = m;
    voceScelta = defaultVoce(m);
  }

  // (Ri)legge lo stato di Piper dal backend.
  function ricaricaPiper() {
    return piperStato()
      .then((st) => {
        piper = st;
        aggiornaMotore();
      })
      .catch(() => {});
  }

  // Scarica l'engine Piper (una volta), mostrando un messaggio di stato.
  async function scaricaEngine() {
    piperAzione = "Scaricamento motore neurale (~26 MB)…";
    try {
      await piperScaricaEngine();
      await ricaricaPiper();
    } catch (e) {
      piperAzione = `Errore: ${e}`;
      return;
    }
    piperAzione = null;
  }

  // Scarica una voce neurale dal catalogo.
  async function scaricaVoce(v) {
    piperAzione = `Scaricamento voce ${v.nome} (~${v.mb} MB)…`;
    try {
      await piperScaricaVoce(v.id);
      await ricaricaPiper();
    } catch (e) {
      piperAzione = `Errore: ${e}`;
      return;
    }
    piperAzione = null;
  }

  // Costruisce i blocchi di lettura: testo per pagina + Alt delle figure.
  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    fermaInterno();
    caricamento = true;
    errore = null;
    blocchi = [];
    indice = 0;

    // Prima prova l'ordine logico dei tag (MCID); se vuoto, ripiega sull'ordine
    // di pagina (testo + Alt delle immagini intercalate).
    blocchiLettura(id)
      .then((tag) => {
        if (annullato) return;
        if (tag && tag.length) {
          ordineLogico = true;
          blocchi = tag.flatMap((b) => {
            const tipo = b.ruolo === "Figure" ? "immagine" : "testo";
            // spezza i blocchi lunghi in frasi per un'evidenziazione piu' fine
            const frasi = tipo === "immagine" ? [b.testo] : inFrasi(b.testo);
            return (frasi.length ? frasi : [b.testo]).map((t) => ({ testo: t, tipo, ruolo: b.ruolo, pagina: b.pagina }));
          });
          caricamento = false;
          return;
        }
        // Fallback: ordine di pagina.
        ordineLogico = false;
        return Promise.all([testoDocumento(id), alberoTag(id).catch(() => null)]).then(([pagine, info]) => {
          if (annullato) return;
          const altPerPagina = {};
          if (info) {
            const cammina = (nodi) => {
              for (const n of nodi) {
                if (n.ruolo === "Figure" && n.alt && n.pagina != null) (altPerPagina[n.pagina] ||= []).push(n.alt);
                cammina(n.figli);
              }
            };
            cammina(info.radice);
          }
          const out = [];
          pagine.forEach((testoPagina, p) => {
            for (const frase of inFrasi(testoPagina)) out.push({ testo: frase, tipo: "testo", pagina: p });
            for (const alt of altPerPagina[p] || []) out.push({ testo: alt, tipo: "immagine", pagina: p });
          });
          blocchi = out;
          caricamento = false;
        });
      })
      .catch((e) => {
        if (!annullato) {
          errore = String(e);
          caricamento = false;
        }
      });

    return () => {
      annullato = true;
      fermaInterno();
    };
  });

  // Carica le voci di sistema (asincrone) e interroga il backend (espeak + Piper).
  $effect(() => {
    ttsInfo()
      .then((info) => {
        espeakDisponibile = info.disponibile;
        espeakVoci = info.voci || [];
        aggiornaMotore();
      })
      .catch(() => aggiornaMotore());
    ricaricaPiper();

    if (sintesi) {
      aggiornaMotore();
      sintesi.addEventListener("voiceschanged", aggiornaMotore);
      return () => sintesi.removeEventListener("voiceschanged", aggiornaMotore);
    }
  });

  function leggiDa(i) {
    if (i >= blocchi.length) {
      inLettura = false;
      return;
    }
    indice = i;
    parolaIndice = 0;
    const b = blocchi[i];
    if (seguiPagina) schede.vaiAPagina(b.pagina);

    const testo = b.tipo === "immagine" ? `Immagine. ${b.testo}` : b.testo;
    if (motore === "espeak" || motore === "piper") {
      leggiBackend(i, testo);
    } else {
      leggiSistema(i, testo, b.tipo);
    }
  }

  // Lettura con le voci di sistema (SpeechSynthesis), con evidenziazione parola.
  function leggiSistema(i, testo, tipo) {
    const u = new SpeechSynthesisUtterance(testo);
    const v = voci.find((x) => x.name === voceScelta);
    if (v) {
      u.voice = v;
      u.lang = v.lang;
    }
    u.rate = velocita;
    const scarto = tipo === "immagine" ? "Immagine. ".length : 0;
    u.onboundary = (e) => {
      if (e.name === "word") parolaIndice = Math.max(0, e.charIndex - scarto);
    };
    u.onend = () => {
      if (inLettura && !inPausa) leggiDa(i + 1);
    };
    u.onerror = () => (inLettura = false);
    sintesi.speak(u);
  }

  // Lettura con un motore backend (espeak o piper): sintetizza un WAV e lo
  // riproduce. Niente evidenziazione per-parola (l'audio non espone i confini).
  async function leggiBackend(i, testo) {
    if (!audio) {
      inLettura = false;
      return;
    }
    try {
      const url =
        motore === "piper"
          ? await piperSintesi(testo, voceScelta, velocita)
          : await ttsSintesi(testo, voceScelta || "it", velocita);
      // Una richiesta piu' recente potrebbe aver cambiato blocco nel frattempo.
      if (!inLettura || inPausa || indice !== i) return;
      audio.src = url;
      audio.onended = () => {
        if (inLettura && !inPausa) leggiDa(i + 1);
      };
      audio.onerror = () => (inLettura = false);
      await audio.play();
    } catch (e) {
      errore = String(e);
      inLettura = false;
    }
  }

  function avvia() {
    if (!blocchi.length || !motore) return;
    if (inPausa) {
      inPausa = false;
      inLettura = true;
      if (motore === "espeak" || motore === "piper") {
        if (audio && audio.src && !audio.ended) audio.play().catch(() => {});
        else leggiDa(indice);
      } else {
        sintesi.resume();
      }
      return;
    }
    fermaInterno();
    inLettura = true;
    inPausa = false;
    leggiDa(indice < blocchi.length ? indice : 0);
  }

  function pausa() {
    inPausa = true;
    if (motore === "espeak" || motore === "piper") audio?.pause();
    else sintesi?.pause();
  }

  function fermaInterno() {
    sintesi?.cancel();
    if (audio) {
      audio.pause();
      audio.removeAttribute("src");
    }
    inLettura = false;
    inPausa = false;
  }

  function ferma() {
    fermaInterno();
    indice = 0;
    parolaIndice = 0;
  }

  function vaiA(i) {
    indice = i;
    if (seguiPagina) schede.vaiAPagina(blocchi[i].pagina);
    if (inLettura || inPausa) {
      fermaInterno();
      avvia();
    }
  }

  // Spezza il blocco corrente in prefisso / parola evidenziata / suffisso.
  function pezzi(testo, off) {
    if (!inLettura && !inPausa) return null;
    const resto = testo.slice(off);
    const m = resto.match(/\S+/);
    if (!m) return null;
    const start = off + m.index;
    const end = start + m[0].length;
    return [testo.slice(0, start), testo.slice(start, end), testo.slice(end)];
  }
</script>

<div class="pannello">
  <header>
    <h3>Lettura vocale</h3>
    <div class="controlli">
      {#if !inLettura || inPausa}
        <button onclick={avvia} disabled={!blocchi.length || !motore}>▶ {inPausa ? "Riprendi" : "Leggi"}</button>
      {:else}
        <button onclick={pausa}>⏸ Pausa</button>
      {/if}
      <button onclick={ferma} disabled={!inLettura && !inPausa}>⏹ Stop</button>
    </div>
    <div class="opzioni">
      <label>
        Motore
        <select value={motore || ""} onchange={(e) => cambiaMotore(e.target.value)} disabled={!motore}>
          {#if voci.length}<option value="sistema">Voci di sistema</option>{/if}
          {#if espeakDisponibile}<option value="espeak">espeak (sintetico)</option>{/if}
          {#if piper.engine_pronto && piperVociInstallate.length}<option value="piper">Piper (neurale)</option>{/if}
          {#if !motore}<option value="">nessuno</option>{/if}
        </select>
      </label>
      <label>
        Voce
        {#if motore === "piper"}
          <select bind:value={voceScelta}>
            {#each piperVociInstallate as v}<option value={v.id}>{v.nome} ({v.lingua}, {v.qualita})</option>{/each}
          </select>
        {:else if motore === "espeak"}
          <select bind:value={voceScelta}>
            {#each espeakVoci as v}<option value={v.codice}>{v.nome} ({v.codice})</option>{/each}
          </select>
        {:else}
          <select bind:value={voceScelta}>
            {#each voci as v}<option value={v.name}>{v.name} ({v.lang})</option>{/each}
          </select>
        {/if}
      </label>
      <label>
        Velocità {velocita.toFixed(1)}×
        <input type="range" min="0.5" max="2" step="0.1" bind:value={velocita} />
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={seguiPagina} />
        Fai seguire il visore
      </label>
    </div>
    {#if blocchi.length}
      <div class="modo">
        Ordine: <b>{ordineLogico ? "logico (tag)" : "pagina"}</b>
        {#if motore}· Motore: <b>{motore === "piper" ? "Piper" : motore === "espeak" ? "espeak" : "sistema"}</b>{/if}
      </div>
    {/if}
  </header>

  {#if motore === "espeak"}
    <p class="info">Voce sintetica <code>espeak-ng</code> (senza evidenziazione per parola). Per una voce naturale scarica <b>Piper</b> qui sotto.</p>
  {:else if !motore}
    <p class="err">Nessuna voce disponibile. Scarica <b>Piper</b> qui sotto, oppure installa <code>espeak-ng</code> / <code>speech-dispatcher</code>.</p>
  {/if}

  {#if piper.supportato}
    <details class="piper">
      <summary>Voce neurale (Piper) — qualità naturale, scaricabile</summary>
      {#if piperAzione}<p class="azione">{piperAzione}</p>{/if}
      {#if !piper.engine_pronto}
        <p class="hint">Scarica una volta il motore neurale, poi una o più voci.</p>
        <button class="mini" onclick={scaricaEngine} disabled={!!piperAzione}>Scarica motore neurale (~26 MB)</button>
      {:else}
        <p class="hint">Motore pronto. Scarica le voci che vuoi usare:</p>
      {/if}
      <ul class="voci-piper">
        {#each piper.voci as v}
          <li>
            <span>{v.nome} <small>({v.lingua}, {v.qualita}, ~{v.mb} MB)</small></span>
            {#if v.installata}
              <span class="ok">✓ installata</span>
            {:else}
              <button class="mini" onclick={() => scaricaVoce(v)} disabled={!piper.engine_pronto || !!piperAzione}>Scarica</button>
            {/if}
          </li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if caricamento}
    <p class="info">Estrazione testo…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if blocchi.length === 0}
    <p class="info">Nessun testo estraibile da questo PDF.</p>
  {:else}
    <ol class="testo">
      {#each blocchi as b, i}
        <li>
          <button
            class="frase"
            class:corrente={i === indice && (inLettura || inPausa)}
            class:immagine={b.tipo === "immagine"}
            onclick={() => vaiA(i)}
          >
            {#if b.tipo === "immagine"}<span class="tag-img">IMG</span>
            {:else if ordineLogico && b.ruolo && b.ruolo !== "P"}<span class="tag-ruolo">{b.ruolo}</span>{/if}
            {#if i === indice && pezzi(b.testo, parolaIndice)}
              {@const p = pezzi(b.testo, parolaIndice)}
              {p[0]}<mark>{p[1]}</mark>{p[2]}
            {:else}
              {b.testo}
            {/if}
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .pannello {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
  }
  header {
    padding: 12px 14px;
    border-bottom: 1px solid var(--bordo);
    position: sticky;
    top: 0;
    background: var(--sfondo);
  }
  h3 {
    margin: 0 0 8px;
    font-size: 15px;
  }
  .controlli {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  .controlli button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
  }
  .controlli button:hover:not(:disabled) {
    border-color: var(--accento);
  }
  .opzioni {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
    color: var(--testo-soft);
  }
  .opzioni label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .opzioni label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .opzioni select,
  .opzioni input {
    accent-color: var(--accento);
  }
  ol.testo {
    margin: 0;
    padding: 8px 14px 24px 32px;
    line-height: 1.5;
  }
  ol.testo li {
    margin: 1px 0;
  }
  button.frase {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: inherit;
    font: inherit;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }
  button.frase:hover {
    background: var(--scheda);
  }
  button.frase.corrente {
    background: var(--accento);
    color: #fff;
  }
  button.frase.immagine {
    color: #7ab0ff;
    font-style: italic;
  }
  .tag-img,
  .tag-ruolo {
    font-size: 10px;
    font-style: normal;
    background: #294a73;
    color: #cfe2ff;
    padding: 0 5px;
    border-radius: 8px;
    margin-right: 6px;
  }
  .tag-ruolo {
    background: #3a3320;
    color: #e7c98a;
  }
  .modo {
    font-size: 11px;
    color: var(--testo-soft);
    margin-top: 6px;
  }
  details.piper {
    margin: 8px 14px;
    border: 1px solid var(--bordo);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 12px;
  }
  details.piper summary {
    cursor: pointer;
    color: var(--testo);
    font-weight: 600;
  }
  details.piper .hint,
  details.piper .azione {
    color: var(--testo-soft);
    margin: 8px 0;
  }
  details.piper .azione {
    color: #e7c98a;
  }
  .voci-piper {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    font-family: inherit;
    font-size: 12px;
  }
  .voci-piper li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 0;
    border-top: 1px solid var(--bordo);
  }
  .voci-piper small {
    color: var(--testo-soft);
  }
  .voci-piper .ok {
    color: #7ad08f;
  }
  button.mini {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 4px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  button.mini:hover:not(:disabled) {
    border-color: var(--accento);
  }
  button.mini:disabled {
    opacity: 0.5;
    cursor: default;
  }
  mark {
    background: #fff3c4;
    color: #000;
    border-radius: 3px;
    padding: 0 1px;
  }
  .info,
  .err {
    padding: 12px 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
  code {
    color: var(--accento);
  }
</style>
