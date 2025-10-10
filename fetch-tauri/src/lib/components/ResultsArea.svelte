<script lang="ts">
  import FileTile from './FileTile.svelte';

  interface FileResult {
    path: string;
    name: string;
    previewUri?: string;
  }

  interface Props {
    results?: FileResult[];
    selectedIndex?: number;
    onselect?: (index: number) => void;
    onopen?: (index: number, path: string) => void;
  }

  let {
    results = [],
    selectedIndex = -1,
    onselect,
    onopen
  }: Props = $props();

  const TILE_WIDTH = 12; // em
  const TILE_HEIGHT = 9; // em

  function handleTileSelect(index: number) {
    onselect?.(index);
  }

  function handleTileOpen(index: number) {
    onopen?.(index, results[index].path);
  }

  // Export function for parent to call on arrow key events
  export function handleArrowKey(direction: 'left' | 'right' | 'up' | 'down') {
    // TODO: Implement arrow key navigation
    // This will need to:
    // 1. Get the computed grid layout from the DOM
    // 2. Calculate the new index based on current position and direction
    // 3. Call onselect with the new index
    console.log('Arrow key navigation:', direction, 'from index', selectedIndex);
  }
</script>

<div class="results-area">
  {#if results.length === 0}
    <div class="empty-state">
      <p>No results to display</p>
    </div>
  {:else}
    <div class="results-grid" style="--tile-width: {TILE_WIDTH}em;">
      {#each results as result, index}
        <FileTile
          fileName={result.name}
          previewUri={result.previewUri}
          selected={index === selectedIndex}
          width={TILE_WIDTH}
          height={TILE_HEIGHT}
          onselect={() => handleTileSelect(index)}
          onopen={() => handleTileOpen(index)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .results-area {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    border: 1px solid var(--color-results-area-border);
    background-color: var(--color-results-area-bg);
    padding: 0.3125em;
  }

  .empty-state {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-placeholder);
    font-size: 1.1em;
  }

  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--tile-width, 12em), 1fr));
    gap: 0.3125em;
    width: 100%;
  }

  /* Custom scrollbar styling */
  .results-area::-webkit-scrollbar {
    width: 0.625em;
  }

  .results-area::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
  }

  .results-area::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 0.3125em;
  }

  .results-area::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.3);
  }
</style>
