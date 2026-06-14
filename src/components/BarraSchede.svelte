<script>
  // Barra delle schede in alto: una scheda per PDF aperto, con bottone chiudi
  // e un "+" per aprire altri documenti.
  import { schede } from "../lib/schede.svelte.js";
</script>

<div class="barra-schede" role="tablist">
  {#each schede.schede as s (s.id)}
    <div
      class="scheda"
      class:attiva={s.id === schede.attiva}
      role="tab"
      tabindex="0"
      aria-selected={s.id === schede.attiva}
      onclick={() => (schede.attiva = s.id)}
      onkeydown={(e) => e.key === "Enter" && (schede.attiva = s.id)}
      title={s.percorso}
    >
      <span class="nome">{s.nome}</span>
      <button
        class="chiudi"
        aria-label="Chiudi scheda"
        onclick={(e) => {
          e.stopPropagation();
          schede.chiudi(s.id);
        }}>×</button
      >
    </div>
  {/each}
  <button class="nuova" aria-label="Apri PDF" onclick={() => schede.apriDaDialogo()}>+</button>
</div>

<style>
  .barra-schede {
    display: flex;
    align-items: stretch;
    gap: 2px;
    background: var(--barra);
    padding: 6px 6px 0 6px;
    overflow-x: auto;
    border-bottom: 1px solid var(--bordo);
  }
  .scheda {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 220px;
    padding: 7px 10px;
    background: var(--scheda);
    color: var(--testo-soft);
    border: 1px solid var(--bordo);
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }
  .scheda.attiva {
    background: var(--sfondo);
    color: var(--testo);
  }
  .nome {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chiudi,
  .nuova {
    background: transparent;
    border: none;
    color: inherit;
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    border-radius: 4px;
    padding: 2px 6px;
  }
  .chiudi:hover {
    background: var(--accento-soft);
  }
  .nuova {
    color: var(--testo-soft);
    align-self: center;
    font-size: 20px;
  }
  .nuova:hover {
    color: var(--accento);
  }
</style>
