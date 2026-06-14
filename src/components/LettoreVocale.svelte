<script>
  // Lettore vocale stile screen reader: legge il testo del PDF nell'ordine delle
  // pagine, frase per frase, evidenziando quella corrente. Usa la sintesi vocale
  // del sistema tramite la Web Speech API del webview (SAPI/AVSpeech/speech-dispatcher).
  import { schede } from "../lib/schede.svelte.js";
  import { testoDocumento } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  const sintesi = typeof window !== "undefined" ? window.speechSynthesis : null;

  let blocchi = $state([]);
  let indice = $state(0);
  let inLettura = $state(false);
  let inPausa = $state(false);
  let caricamento = $state(false);
  let errore = $state(null);

  let voci = $state([]);
  let voceScelta = $state("");
  let velocita = $state(1);

  // Spezza il testo in frasi leggibili.
  function inFrasi(testo) {
    return (testo.match(/[^.!?\n]+[.!?]*/g) || [])
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
  }

  function caricaVoci() {
    if (!sintesi) return;
    voci = sintesi.getVoices();
    if (!voceScelta && voci.length) {
      // Preferisci una voce italiana se disponibile.
      const it = voci.find((v) => v.lang.toLowerCase().startsWith("it"));
      voceScelta = (it || voci[0]).name;
    }
  }

  // Carica il testo della scheda attiva e resetta la lettura.
  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    fermaInterno();
    caricamento = true;
    errore = null;
    blocchi = [];
    indice = 0;
    testoDocumento(id)
      .then((pagine) => {
        if (annullato) return;
        blocchi = inFrasi(pagine.join("\n"));
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
    const u = new SpeechSynthesisUtterance(blocchi[i]);
    const v = voci.find((x) => x.name === voceScelta);
    if (v) {
      u.voice = v;
      u.lang = v.lang;
    }
    u.rate = velocita;
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
  }

  function vaiA(i) {
    indice = i;
    if (inLettura || inPausa) {
      fermaInterno();
      avvia();
    }
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
            onclick={() => vaiA(i)}>{b}</button>
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
