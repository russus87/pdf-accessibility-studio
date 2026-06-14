<script>
  // Shell dell'applicazione: barra schede, toolbar, visore e banner errori.
  import { schede } from "./lib/schede.svelte.js";
  import BarraSchede from "./components/BarraSchede.svelte";
  import BarraStrumenti from "./components/BarraStrumenti.svelte";
  import Visore from "./components/Visore.svelte";
</script>

<main>
  <BarraSchede />
  <BarraStrumenti />

  {#if schede.errore}
    <div class="banner-errore" role="alert">
      {schede.errore}
      <button onclick={() => (schede.errore = null)} aria-label="Chiudi avviso">×</button>
    </div>
  {/if}

  <Visore />
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .banner-errore {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    background: var(--errore-sfondo);
    color: var(--errore);
    padding: 8px 14px;
    font-size: 13px;
  }
  .banner-errore button {
    background: transparent;
    border: none;
    color: inherit;
    font-size: 16px;
    cursor: pointer;
  }
</style>
