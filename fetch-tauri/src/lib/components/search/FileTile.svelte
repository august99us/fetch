<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";

  interface FileResult {
    path: string;
    name: string;
  }

  interface Props {
    file: FileResult;
    selected?: boolean;
    width?: number;
    height?: number;
    onselect?: () => void;
    onopen?: () => void;
    onhover?: () => void;
  }

  let {
    file,
    selected = false,
    width = 12,
    height = 9,
    onselect,
    onopen,
    onhover
  }: Props = $props();

  let previewUriPromise = $derived(previewPath(file.path))

  let buttonElement: HTMLButtonElement | undefined = $state();

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

  export function scrollIntoView(options?: ScrollIntoViewOptions) {
    buttonElement?.scrollIntoView(options);
  }

  async function previewPath(path: string): Promise<string> {
    let previewPath: string = await invoke("preview", { path });
    if (previewPath) {
      return convertFileSrc(previewPath);
    } else {
      return '/broken.png';
    }
  }
</script>

<button
  bind:this={buttonElement}
  class="file-tile"
  class:selected
  class:hovered={!selected}
  style="width: {width}rem; height: {height}rem;"
  onclick={handleClick}
  ondblclick={handleDoubleClick}
  onmouseenter={handleMouseEnter}
>
  <div class="preview-container">
    {#await previewUriPromise}
      <img src="/placeholder.png" alt={file.name} class="preview-image" />
    {:then previewUri} 
      <img src={previewUri} alt={file.name} class="preview-image" />
    {:catch error}
      <img src="/broken.png" alt={file.name} class="preview-image" />
    {/await}
  </div>
  <div class="file-name">{file.name}</div>
</button>

<style>
  .file-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1rem;
    margin: 0;
    border: 0;
    background-color: var(--color-item-bg-default);
    cursor: pointer;
    transition: all 0.15s ease;
    font-family: inherit;
    color: var(--color-text);
    box-sizing: border-box;
  }

  .file-tile:hover {
    background-color: var(--color-item-bg-hover);
    border-color: var(--color-item-border-hover);
  }

  .file-tile.selected {
    background-color: var(--color-item-bg-selected);
    border-color: var(--color-item-border-selected);
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
    padding-top: 0.5rem;
    font-size: 1em;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>