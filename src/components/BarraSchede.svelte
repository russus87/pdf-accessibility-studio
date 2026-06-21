<script>
  // Barra delle schede in alto: una scheda per documento aperto (PDF o
  // "Nuovo da modello"), con icona di tipo, bottone chiudi e i comandi per
  // aprire un PDF o creare un nuovo documento.
  import { schede } from "../lib/schede.svelte.js";
  import Icona from "./Icona.svelte";
</script>

<div class="barra-schede" role="tablist">
  <div class="lista">
    {#each schede.schede as s (s.id)}
      <div
        class="scheda"
        class:attiva={s.id === schede.attiva}
        role="tab"
        tabindex="0"
        aria-selected={s.id === schede.attiva}
        onclick={() => (schede.attiva = s.id)}
        onkeydown={(e) => e.key === "Enter" && (schede.attiva = s.id)}
        title={s.percorso || s.nome}
      >
        <span class="ico"><Icona nome={s.tipo === "creatore" ? "file-plus" : "file"} dim={14} /></span>
        <span class="nome">{s.nome}</span>
        <button
          class="chiudi"
          aria-label="Chiudi scheda"
          onclick={(e) => {
            e.stopPropagation();
            schede.chiudi(s.id);
          }}><Icona nome="x" dim={13} /></button
        >
      </div>
    {/each}
  </div>

  <div class="comandi">
    <button class="azione" aria-label="Apri PDF" title="Apri PDF (Ctrl+O)" onclick={() => schede.apriDaDialogo()}>
      <Icona nome="folder-open" dim={15} />
    </button>
    <button class="azione" aria-label="Nuovo da modello" title="Nuovo da modello" onclick={() => schede.nuovoCreatore()}>
      <Icona nome="plus" dim={16} />
    </button>
  </div>
</div>

<style>
  .barra-schede {
    display: flex;
    align-items: stretch;
    background: var(--bg-attivita);
    border-bottom: 1px solid var(--bordo);
    padding-left: 4px;
  }
  .lista {
    display: flex;
    align-items: stretch;
    gap: 1px;
    overflow-x: auto;
    flex: 1;
    min-width: 0;
  }
  .lista::-webkit-scrollbar {
    height: 0;
  }
  .scheda {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 230px;
    padding: 0 10px;
    height: 36px;
    background: transparent;
    color: var(--testo-soft);
    border: none;
    border-top: 2px solid transparent;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    transition: background var(--transizione), color var(--transizione);
  }
  .scheda:hover {
    background: var(--hover);
    color: var(--testo);
  }
  .scheda.attiva {
    background: var(--tela);
    color: var(--testo);
    border-top-color: var(--accento);
  }
  .ico {
    display: flex;
    color: var(--testo-soft);
    flex: none;
  }
  .scheda.attiva .ico {
    color: var(--accento-testo);
  }
  .nome {
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
  }
  .chiudi {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--testo-soft);
    cursor: pointer;
    border-radius: var(--r-1);
    padding: 3px;
    opacity: 0.7;
  }
  .chiudi:hover {
    background: var(--attivo);
    color: var(--testo);
    opacity: 1;
  }
  .comandi {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px;
    flex: none;
  }
  .azione {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    background: transparent;
    border: none;
    color: var(--testo-soft);
    cursor: pointer;
    border-radius: var(--r-2);
    transition: background var(--transizione), color var(--transizione);
  }
  .azione:hover {
    background: var(--hover);
    color: var(--accento-testo);
  }
</style>
