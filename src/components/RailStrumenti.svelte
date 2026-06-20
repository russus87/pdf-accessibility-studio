<script>
  // Rail verticale a sinistra: interruttori degli strumenti, raggruppati per
  // categoria. Ogni voce apre/chiude un pannello (a destra del visore) o, nel
  // caso delle anteprime, la striscia accanto al visore.
  import { schede } from "../lib/schede.svelte.js";

  const s = $derived(schede.schedaAttiva);
</script>

<nav class="rail" aria-label="Strumenti">
  <div class="gruppo">
    <span class="titolo">Crea</span>
    <button class:on={schede.pannello === "crea"}
      onclick={() => schede.mostraPannello("crea")}>
      <span class="icona" aria-hidden="true">📝</span><span>Nuovo da modello</span>
    </button>
    <button class:on={schede.pannello === "editor"} disabled={!s}
      onclick={() => schede.mostraPannello("editor")}>
      <span class="icona" aria-hidden="true">✏️</span><span>Editor PDF</span>
    </button>
  </div>

  <div class="gruppo">
    <span class="titolo">Documento</span>
    <button class:on={schede.anteprime} disabled={!s}
      onclick={() => (schede.anteprime = !schede.anteprime)}>
      <span class="icona" aria-hidden="true">🖼</span><span>Anteprime</span>
    </button>
    <button class:on={schede.pannello === "pagine"} disabled={!s}
      onclick={() => schede.mostraPannello("pagine")}>
      <span class="icona" aria-hidden="true">📄</span><span>Pagine</span>
    </button>
    <button class:on={schede.pannello === "metadati"} disabled={!s}
      onclick={() => schede.mostraPannello("metadati")}>
      <span class="icona" aria-hidden="true">ℹ</span><span>Metadati</span>
    </button>
  </div>

  <div class="gruppo">
    <span class="titolo">Accessibilità</span>
    <button class:on={schede.pannello === "valida"} disabled={!s}
      onclick={() => schede.mostraPannello("valida")}>
      <span class="icona" aria-hidden="true">✓</span><span>Valida</span>
    </button>
    <button class:on={schede.pannello === "correggi"} disabled={!s}
      onclick={() => schede.mostraPannello("correggi")}>
      <span class="icona" aria-hidden="true">✎</span><span>Correggi</span>
    </button>
    <button class:on={schede.pannello === "tag"} disabled={!s}
      onclick={() => schede.mostraPannello("tag")}>
      <span class="icona" aria-hidden="true">🏷</span><span>Tag</span>
    </button>
    <button class:on={schede.pannello === "autotag"} disabled={!s}
      onclick={() => schede.mostraPannello("autotag")}>
      <span class="icona" aria-hidden="true">✨</span><span>Auto-tag</span>
    </button>
    <button class:on={schede.pannello === "moduli"} disabled={!s}
      onclick={() => schede.mostraPannello("moduli")}>
      <span class="icona" aria-hidden="true">🧾</span><span>Moduli</span>
    </button>
    <button class:on={schede.pannello === "indice"} disabled={!s}
      onclick={() => schede.mostraPannello("indice")}>
      <span class="icona" aria-hidden="true">🔖</span><span>Indice</span>
    </button>
    <button class:on={schede.pannello === "leggi"} disabled={!s}
      onclick={() => schede.mostraPannello("leggi")}>
      <span class="icona" aria-hidden="true">🔊</span><span>Leggi</span>
    </button>
    <button class:on={schede.pannello === "ai"} disabled={!s}
      onclick={() => schede.mostraPannello("ai")}>
      <span class="icona" aria-hidden="true">🤖</span><span>AI documento</span>
    </button>
  </div>

  <div class="gruppo">
    <span class="titolo">Strumenti</span>
    <button class:on={schede.pannello === "cerca"} disabled={!s}
      onclick={() => schede.mostraPannello("cerca")}>
      <span class="icona" aria-hidden="true">🔍</span><span>Cerca</span>
    </button>
    <button class:on={schede.pannello === "strumenti"} disabled={!s}
      onclick={() => schede.mostraPannello("strumenti")}>
      <span class="icona" aria-hidden="true">🛠</span><span>Strumenti PDF</span>
    </button>
    <button class:on={schede.pannello === "libreria"}
      onclick={() => schede.mostraPannello("libreria")}>
      <span class="icona" aria-hidden="true">📚</span><span>Libreria</span>
    </button>
    <button class:on={schede.misura} disabled={!s}
      onclick={() => (schede.misura = !schede.misura)}>
      <span class="icona" aria-hidden="true">📏</span><span>Misura</span>
    </button>
    <button class:on={schede.pannello === "confronta"} disabled={schede.schede.length < 2}
      onclick={() => schede.mostraPannello("confronta")}>
      <span class="icona" aria-hidden="true">⇆</span><span>Confronta</span>
    </button>
  </div>
</nav>

<style>
  .rail {
    width: 168px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 8px;
    background: var(--barra);
    border-right: 1px solid var(--bordo);
    overflow-y: auto;
  }
  .gruppo {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .gruppo + .gruppo {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--bordo);
  }
  .titolo {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--testo-soft);
    padding: 2px 8px 4px;
  }
  .rail button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    text-align: left;
    background: transparent;
    color: var(--testo);
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 7px 9px;
    cursor: pointer;
    font-size: 13px;
  }
  .rail button:hover:not(:disabled) {
    background: var(--scheda);
    border-color: var(--bordo);
  }
  .rail button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .rail button.on {
    background: var(--accento);
    color: #fff;
    border-color: var(--accento);
  }
  .icona {
    width: 18px;
    text-align: center;
    flex: none;
  }
</style>
