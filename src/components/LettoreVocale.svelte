<script>
  // Lettore vocale stile screen reader. Legge il testo nell'ordine delle pagine
  // e, dove il PDF e' taggato, intercala il testo alternativo delle immagini.
  // Evidenzia la parola in lettura (eventi onboundary) e fa seguire il visore.
  import { schede } from "../lib/schede.svelte.js";
  import { testoDocumento, alberoTag } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  const sintesi = typeof window !== "undefined" ? window.speechSynthesis : null;

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

  function inFrasi(testo) {
    return (testo.match(/[^.!?\n]+[.!?]*/g) || []).map((t) => t.trim()).filter(Boolean);
  }

  function caricaVoci() {
    if (!sintesi) return;
    voci = sintesi.getVoices();
    if (!voceScelta && voci.length) {
      const it = voci.find((v) => v.lang.toLowerCase().startsWith("it"));
      voceScelta = (it || voci[0]).name;
    }
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

    Promise.all([testoDocumento(id), alberoTag(id).catch(() => null)])
      .then(([pagine, info]) => {
        if (annullato) return;
        // Mappa pagina -> testi Alt delle figure.
        const altPerPagina = {};
        if (info) {
          const cammina = (nodi) => {
            for (const n of nodi) {
              if (n.ruolo === "Figure" && n.alt && n.pagina != null) {
                (altPerPagina[n.pagina] ||= []).push(n.alt);
              }
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
      })
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));

    return () => {
      annullato = true;
      fermaInterno();
    };
  });

  $effect(() => {
    if (!sintesi) return;
    caricaVoci();
    sintesi.addEventListener("voiceschanged", caricaVoci);
    return () => sintesi.removeEventListener("voiceschanged", caricaVoci);
  });

  function leggiDa(i) {
    if (!sintesi || i >= blocchi.length) {
      inLettura = false;
      return;
    }
    indice = i;
    parolaIndice = 0;
    const b = blocchi[i];
    if (seguiPagina) schede.vaiAPagina(b.pagina);

    const testo = b.tipo === "immagine" ? `Immagine. ${b.testo}` : b.testo;
    const u = new SpeechSynthesisUtterance(testo);
    const v = voci.find((x) => x.name === voceScelta);
    if (v) {
      u.voice = v;
      u.lang = v.lang;
    }
    u.rate = velocita;
    const scarto = b.tipo === "immagine" ? "Immagine. ".length : 0;
    u.onboundary = (e) => {
      if (e.name === "word") parolaIndice = Math.max(0, e.charIndex - scarto);
    };
    u.onend = () => {
      if (inLettura && !inPausa) leggiDa(i + 1);
    };
    u.onerror = () => (inLettura = false);
    sintesi.speak(u);
  }

  function avvia() {
    if (!sintesi || !blocchi.length) return;
    if (inPausa) {
      sintesi.resume();
      inPausa = false;
      inLettura = true;
      return;
    }
    sintesi.cancel();
    inLettura = true;
    inPausa = false;
    leggiDa(indice < blocchi.length ? indice : 0);
  }

  function pausa() {
    if (!sintesi) return;
    sintesi.pause();
    inPausa = true;
  }

  function fermaInterno() {
    if (!sintesi) return;
    sintesi.cancel();
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
        <button onclick={avvia} disabled={!blocchi.length}>▶ {inPausa ? "Riprendi" : "Leggi"}</button>
      {:else}
        <button onclick={pausa}>⏸ Pausa</button>
      {/if}
      <button onclick={ferma} disabled={!inLettura && !inPausa}>⏹ Stop</button>
    </div>
    <div class="opzioni">
      <label>
        Voce
        <select bind:value={voceScelta}>
          {#each voci as v}<option value={v.name}>{v.name} ({v.lang})</option>{/each}
        </select>
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
  </header>

  {#if !sintesi}
    <p class="err">La sintesi vocale non è disponibile in questo webview.</p>
  {:else if voci.length === 0}
    <p class="info">Nessuna voce di sistema trovata. Su Linux installa <code>speech-dispatcher</code> e una voce (es. <code>espeak-ng</code>).</p>
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
            {#if b.tipo === "immagine"}<span class="tag-img">IMG</span>{/if}
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
  .tag-img {
    font-size: 10px;
    font-style: normal;
    background: #294a73;
    color: #cfe2ff;
    padding: 0 5px;
    border-radius: 8px;
    margin-right: 6px;
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
