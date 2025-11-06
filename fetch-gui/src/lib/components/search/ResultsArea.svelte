<script lang="ts">
  import FileTile from './FileTile.svelte';
  import SpinnerBar from '../common/SpinnerBar.svelte';
  const TILE_WIDTH = 20; // rem
  const TILE_HEIGHT = 15; // rem

  interface FileResult {
    path: string;
    name: string;
  }

  interface Props {
    results: FileResult[];
    loading?: boolean;
    selectedIndex?: number;
    onselect?: (index: number) => void;
    onopen?: (index: number, path: string) => void;
  }

  let {
    results,
    loading = false,
    selectedIndex = $bindable(-1),
    onselect,
    onopen
  }: Props = $props();

  let gridContainer: HTMLDivElement | undefined = $state();

  function handleTileSelect(index: number) {
    selectedIndex = index;

    onselect?.(index);
  }

  function handleTileOpen(index: number, path: string) {
    onopen?.(index, path);
  }

  // Export function for parent to call on arrow key events
  export function handleArrowKey(direction: 'left' | 'right' | 'up' | 'down') {
    if (results.length === 0 || !gridContainer) return;

    let newIndex;
    
    if (selectedIndex === -1) {
      // If nothing is selected, select the first item
      newIndex = 0;
    } else {
      // Get the computed grid layout
      const computedStyle = window.getComputedStyle(gridContainer);
      const gridTemplateColumns = computedStyle.getPropertyValue('grid-template-columns');

      // Count columns by splitting the template (each column width is a space-separated value)
      const columnsPerRow = gridTemplateColumns.split(' ').length;

      newIndex = selectedIndex;
      switch (direction) {
        case 'left':
          if (selectedIndex % columnsPerRow !== 0) {
            newIndex = selectedIndex - 1;
          }
          break;
        case 'right':
          if ((selectedIndex + 1) % columnsPerRow !== 0 && selectedIndex + 1 < results.length) {
            newIndex = selectedIndex + 1;
          }
          break;
        case 'up':
          newIndex = selectedIndex - columnsPerRow;
          if (newIndex < 0) newIndex = selectedIndex;
          break;
        case 'down':
          newIndex = selectedIndex + columnsPerRow;
          if (newIndex >= results.length) newIndex = selectedIndex;
          break;
      }
    }

    if (newIndex !== selectedIndex) {
      handleTileSelect(newIndex);
    }
  }
</script>

<div class="results-area">
  <div class="results-grid" class:disabled={(results.length == 0) && loading} style="--tile-width: {TILE_WIDTH}rem;" bind:this={gridContainer}>
    {#each results as result, index}
      <FileTile
        file={result}
        selected={index === selectedIndex}
        width={TILE_WIDTH}
        height={TILE_HEIGHT}
        onselect={() => handleTileSelect(index)}
        onopen={() => handleTileOpen(index, result.path)}
      />
    {/each}
  </div>
  {#if loading}
    <SpinnerBar />
  {/if}
</div>

<style>
  .results-area {
    width: 100%;
    height: 100%;
    max-height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0.5rem;
    border: 0;
    background-color: var(--color-results-area-bg);
  }

  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, var(--tile-width, 20rem));
    gap: 0.5rem;
    justify-content: center;
  }
  
  .results-grid.disabled {
    pointer-events: none;
    opacity: 0.6;
    filter: grayscale(100%);
  }

  /* Custom scrollbar styling */
  .results-area::-webkit-scrollbar {
    width: 0.5rem;
  }

  .results-area::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
  }

  .results-area::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 0.3125rem;
  }

  .results-area::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.3);
  }
</style>
