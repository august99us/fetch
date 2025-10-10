<script lang="ts">
  interface Props {
    fileName: string;
    previewUri?: string;
    selected?: boolean;
    width?: number;
    height?: number;
    onselect?: () => void;
    onopen?: () => void;
    onhover?: () => void;
  }

  let {
    fileName,
    previewUri = '/placeholder.png',
    selected = false,
    width = 12,
    height = 9,
    onselect,
    onopen,
    onhover
  }: Props = $props();

  function handleClick() {
    onselect?.();
  }

  function handleDoubleClick() {
    onopen?.();
  }

  function handleMouseEnter() {
    if (!selected) {
      onhover?.();
    }
  }
</script>

<button
  class="file-tile"
  class:selected
  class:hovered={!selected}
  style="width: {width}em; height: {height}em;"
  onclick={handleClick}
  ondblclick={handleDoubleClick}
  onmouseenter={handleMouseEnter}
>
  <div class="preview-container">
    <img src={previewUri} alt={fileName} class="preview-image" />
  </div>
  <div class="file-name">{fileName}</div>
</button>

<style>
  .file-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0.25em;
    border: 1px solid var(--color-tile-border-default);
    background-color: var(--color-tile-bg-default);
    cursor: pointer;
    transition: all 0.15s ease;
    overflow: hidden;
    font-family: inherit;
    color: var(--color-text);
  }

  .file-tile:hover {
    background-color: var(--color-tile-bg-hover);
    border-color: var(--color-tile-border-hover);
  }

  .file-tile.selected {
    background-color: var(--color-tile-bg-selected);
    border-color: var(--color-tile-border-selected);
  }

  .preview-container {
    flex: 1;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  .file-name {
    width: 100%;
    padding-top: 0.25em;
    font-size: 0.9em;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>