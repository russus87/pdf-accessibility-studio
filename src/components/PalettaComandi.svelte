<script>
  // Palette dei comandi (Ctrl+K): cerca ed esegue rapidamente qualsiasi azione
  // — apertura pannelli, viste centrali, file. Stile VS Code.
  import { schede } from "../lib/schede.svelte.js";

  let aperta = $state(false);
  let filtro = $state("");
  let sel = $state(0);
  let inputEl = $state(null);

  // Definizione dei comandi: etichetta, eventuale scorciatoia, azione e
  // disponibilità (alcuni richiedono un PDF aperto o due per il confronto).
  const haPdf = () => !!schede.pdfAttivo;
  const base = [
    { eti: "Procedura guidata di remediation", run: () => (schede.guida = true) },
    { eti: "Apri PDF…", sc: "Ctrl+O", run: () => schede.apriDaDialogo() },
    { eti: "Nuovo documento da modello", run: () => schede.nuovoCreatore() },
    { eti: "Chiudi scheda", sc: "Ctrl+W", ok: () => !!schede.attiva, run: () => schede.attiva && schede.chiudi(schede.attiva) },
    { eti: "Vista: Editor (modifica PDF)", ok: haPdf, run: () => (schede.modoCentro = "editor") },
    { eti: "Vista: Confronta due PDF", ok: () => schede.numeroPdf >= 2, run: () => (schede.modoCentro = "confronta") },
    { eti: "Vista: Visore", ok: haPdf, run: () => (schede.modoCentro = null) },
    { eti: "Righello e misura (attiva/disattiva)", ok: haPdf, run: () => (schede.misura = !schede.misura) },
    { eti: "Pannello: Anteprime", ok: haPdf, run: () => schede.apriPannello("anteprime") },
    { eti: "Pannello: Pagine", ok: haPdf, run: () => schede.apriPannello("pagine") },
    { eti: "Pannello: Segnalibri / Indice", ok: haPdf, run: () => schede.apriPannello("indice") },
    { eti: "Pannello: Metadati", ok: haPdf, run: () => schede.apriPannello("metadati") },
    { eti: "Pannello: Cerca", sc: "Ctrl+F", ok: haPdf, run: () => schede.apriPannello("cerca") },
    { eti: "Pannello: Validazione accessibilità", ok: haPdf, run: () => schede.apriPannello("valida") },
    { eti: "Pannello: Correzione", ok: haPdf, run: () => schede.apriPannello("correggi") },
    { eti: "Pannello: Struttura / Tag", ok: haPdf, run: () => schede.apriPannello("tag") },
    { eti: "Tag: Testo alternativo (AI)", ok: haPdf, run: () => schede.apriPannello("tag", "alt") },
    { eti: "Tag: Ordine di lettura", ok: haPdf, run: () => schede.apriPannello("tag", "riordina") },
    { eti: "Pannello: Auto-tag", ok: haPdf, run: () => schede.apriPannello("autotag") },
    { eti: "Anteprima lettura (screen reader)", ok: haPdf, run: () => schede.apriPannello("ordine") },
    { eti: "Pannello: Moduli", ok: haPdf, run: () => schede.apriPannello("moduli") },
    { eti: "Pannello: Lettura vocale", ok: haPdf, run: () => schede.apriPannello("leggi") },
    { eti: "Pannello: AI", ok: haPdf, run: () => schede.apriPannello("ai") },
    { eti: "Pannello: Strumenti", ok: haPdf, run: () => schede.apriPannello("strumenti") },
    { eti: "Pannello: Libreria", run: () => schede.apriPannello("libreria") },
  ];

  const visibili = $derived.by(() => {
    const q = filtro.trim().toLowerCase();
    return base
      .filter((c) => (c.ok ? c.ok() : true))
      .filter((c) => !q || c.eti.toLowerCase().includes(q));
  });

  function apri() {
    aperta = true;
    filtro = "";
    sel = 0;
  }
  function chiudi() {
    aperta = false;
  }
  function esegui(c) {
    chiudi();
    c?.run();
  }

  // Apre la palette con Ctrl/Cmd+K da qualsiasi punto.
  $effect(() => {
    function onKey(e) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        aperta ? chiudi() : apri();
      } else if (e.key === "Escape" && aperta) {
        chiudi();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // Porta il fuoco all'input quando si apre.
  $effect(() => {
    if (aperta && inputEl) inputEl.focus();
  });

  // Mantiene la selezione dentro i limiti dei risultati.
  $effect(() => {
    if (sel >= visibili.length) sel = Math.max(0, visibili.length - 1);
  });

  function suTasti(e) {
    if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(sel + 1, visibili.length - 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(sel - 1, 0); }
    else if (e.key === "Enter") { e.preventDefault(); esegui(visibili[sel]); }
  }
</script>

{#if aperta}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="velo" onclick={chiudi}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="paletta" onclick={(e) => e.stopPropagation()}>
      <input
        bind:this={inputEl}
        bind:value={filtro}
        onkeydown={suTasti}
        placeholder="Cerca un comando…"
        aria-label="Cerca un comando"
      />
      <ul>
        {#each visibili as c, i}
          <li class:sel={i === sel}>
            <button onmouseenter={() => (sel = i)} onclick={() => esegui(c)}>
              <span class="eti">{c.eti}</span>
              {#if c.sc}<span class="sc">{c.sc}</span>{/if}
            </button>
          </li>
        {:else}
          <li class="vuoto">Nessun comando</li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style>
  .velo {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 1000;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
  }
  .paletta {
    width: 560px;
    max-width: 90vw;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--barra, #1e1e1e);
    border: 1px solid var(--bordo);
    border-radius: 10px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  input {
    border: none;
    border-bottom: 1px solid var(--bordo);
    background: transparent;
    color: var(--testo);
    padding: 14px 16px;
    font-size: 15px;
    outline: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow-y: auto;
  }
  li.vuoto {
    padding: 14px 12px;
    color: var(--testo-soft);
    font-size: 13px;
  }
  li button {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    background: transparent;
    border: none;
    color: var(--testo);
    text-align: left;
    padding: 9px 12px;
    border-radius: 7px;
    cursor: pointer;
    font-size: 13.5px;
  }
  li.sel button {
    background: var(--accento);
    color: #fff;
  }
  .sc {
    font-size: 11px;
    color: var(--testo-soft);
    font-variant-numeric: tabular-nums;
  }
  li.sel .sc {
    color: rgba(255, 255, 255, 0.85);
  }
</style>
