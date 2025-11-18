<script lang="ts">
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

  interface Props {
    isOpen?: boolean;
    ontoggle?: (isOpen: boolean) => void;
    onindex?: (folders: string[]) => void;
    onselectfolders?: () => void;
  }

  let {
    isOpen = $bindable(false),
    ontoggle,
    onindex,
  }: Props = $props();

  // Maximum characters to keep in log (prevents unbounded growth)
  // Targetted effective line count of 500 (150 characters per line)
  const MAX_LOG_CHARS = 75000;

  let drawerWidth = $state.raw(1200);
  let stagedPaths = $state<string[]>([]);
  let selectedPaths = $state<string[]>([]);
  let indexing = $state(false);
  let progressCurrent = $state(0);
  let progressTotal = $state(0);
  let logs = $state<string>('');
  let logTextarea: HTMLTextAreaElement | undefined = $state();

  interface ProgressEvent {
    current: number,
    total: number,
  }
  interface LogEvent {
    message: string,
  }

  let unlistenProgress: (() => void) | undefined;
  let unlistenLog: (() => void) | undefined;
  async function listenEvents() {
    const appWebview = getCurrentWebviewWindow();
    unlistenProgress = await appWebview.listen<ProgressEvent>('index_progress', (event) => {
      progressCurrent = event.payload.current;
      progressTotal = event.payload.total;
    });
    unlistenLog = await appWebview.listen<LogEvent>('index_log', (event) => {
      // Append new log with newline
      logs += (logs.length > 0 ? '\n' : '') + event.payload.message;

      // Simple trim from beginning if exceeds max
      if (logs.length > MAX_LOG_CHARS) {
        logs = logs.substring(logs.length - MAX_LOG_CHARS);
      }

      if (logTextarea) {
        logTextarea.scrollTop = logTextarea.scrollHeight;
      }
    });
  }

  async function unlistenEvents() {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = undefined;
    }
    if (unlistenLog) {
      unlistenLog();
      unlistenLog = undefined;
    }
  }

  async function handleToggle() {
    isOpen = !isOpen;
    if (isOpen) {
      // Opening
      await listenEvents();
    } else {
      // Closing
      stagedPaths = [];
      selectedPaths = [];
      indexing = false;
      progressCurrent = 0;
      progressTotal = 0;
      await unlistenEvents();
    }
    ontoggle?.(isOpen);
  }

  async function handleIndex() {
    logs = '';
    indexing = true;
    try {
      await invoke('index', { paths: stagedPaths });
    } catch (e) {
      console.error("Error during indexing:", e);
    }
    onindex?.(stagedPaths);
    console.log("Finished indexing");
    setTimeout(() => {
      handleToggle();
      indexing = false;
    }, 2000);
  }

  async function handleSelectFiles() {
    const paths: string[] | null = await open({
      multiple: true,
      directory: false,
      title: "Select Files to Index"
    });

    if (paths) {
      for (let path of paths) {
        if (!stagedPaths.includes(path)) {
          stagedPaths.push(path);
        }
      }
    }
  }

  async function handleSelectFolders() {
    const paths: string[] | null = await open({
      multiple: true,
      directory: true,
      title: "Select Folders to Index"
    });

    if (paths) {
      for (let path of paths) {
        if (!stagedPaths.includes(path)) {
          stagedPaths.push(path);
        }
      }
    }
  }

  function handleRemove() {
    stagedPaths = stagedPaths.filter(path => !selectedPaths.includes(path));
    selectedPaths = [];
  }
</script>

<div>
  <!-- spacer (slightly smaller for gap additions) -->
  <div style="height: 2rem;">&nbsp;</div>
  <div class="index-drawer" bind:clientWidth={drawerWidth} class:open={isOpen}>
    <button class="drawer-bar" disabled={indexing} onclick={handleToggle}>
      {#if isOpen}
        <span class="bar-text left" transition:fly={{ x: drawerWidth, opacity: -25, easing: cubicOut, duration: 220 }}>&lt; Back to File Search</span>
      {/if}
      <!-- Can't use else here because both elements need to exist at once for animation -->
      {#if !isOpen}
        <span class="bar-text right underlined" transition:fly={{ x: -drawerWidth, opacity: -25, easing: cubicOut, duration: 220 }}>Index</span>
      {/if}
    </button>

    <div class="drawer-content">
      <select
        class="path-list"
        multiple
        disabled={indexing}
        bind:value={selectedPaths}
      >
        {#if stagedPaths.length === 0}
          <option disabled>No paths staged yet - click "Select Files/Folders..." to add</option>
        {:else}
          {#each stagedPaths as path}
            <option value={path}>{path}</option>
          {/each}
        {/if}
      </select>

      <div class="button-group">
        <button class="secondary-button" disabled={indexing} onclick={handleSelectFiles}>
          Select Files...
        </button>
        <button class="secondary-button" disabled={indexing} onclick={handleSelectFolders}>
          Select Folders...
        </button>
        <button class="secondary-button" disabled={indexing} onclick={handleRemove}>
          Remove
        </button>
        <button class="primary-button" disabled={indexing} onclick={handleIndex}>
          Index
        </button>
      </div>
      
      <div>&nbsp;</div>

      <div class="progress-section">
        <div class="progress-bar-container">
          <div class="progress-bar">
            <div
              class="progress-bar-fill"
              style="width: {progressTotal > 0 ? (progressCurrent / progressTotal * 100) : 0}%"
            ></div>
          </div>
          <span class="progress-text">{progressCurrent}/{progressTotal}</span>
        </div>

        <textarea
          bind:this={logTextarea}
          class="log-display"
          readonly
          value={logs}
        ></textarea>
      </div>
    </div>
  </div>
</div>

<style>
  .index-drawer {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 3rem;
    transition: height 0.2s ease;
    background-color: var(--color-background);
    z-index: 100;
  }

  .index-drawer.open {
    height: 100vh;
  }

  .drawer-bar {
    width: 100%;
    height: 3rem;
    background-color: var(--color-section-bg);
    border: none;
    border-top: 1px solid var(--color-section-border);
    border-bottom: 1px solid var(--color-section-border);
    cursor: pointer;
    display: flex;
    align-items: center;
    padding: 0 1.5rem;
    transition: all 0.1s ease;
  }

  .drawer-bar:hover:not(:disabled) {
    background-color: var(--color-section-bg-hover);
  }

  .drawer-bar:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .bar-text {
    font-family: inherit;
    font-size: 1.1em;
    color: var(--color-text);
  }

  .bar-text.left {
    text-align: left;
    margin-right: auto;
  }

  .bar-text.right {
    text-align: right;
    margin-left: auto;
  }

  .bar-text.underlined {
    text-decoration: underline;
  }

  .drawer-content {
    display: flex;
    flex-direction: column;
    height: calc(100% - 3rem);
    padding: 1.5rem;
    gap: 1rem;
    background-color: var(--color-background);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s ease;
  }

  .index-drawer.open .drawer-content {
    opacity: 1;
    pointer-events: auto;
  }

  .path-list {
    flex: 1;
    width: 100%;
    max-height: 20rem;
    padding: 1rem;
    font-family: monospace;
    font-size: 1.1em;
    border: 1px solid var(--color-input-border);
    background-color: var(--color-input-bg);
    color: var(--color-text);
    border-radius: 0.5rem;
    box-sizing: border-box;
    transition: all 0.15s ease;
    overflow-y: auto;
  }

  .path-list:focus {
    border-color: var(--color-input-border-focus);
    background-color: var(--color-input-bg-focus);
  }

  .path-list option {
    cursor: pointer;
  }

  .path-list option:checked {
    background-color: var(--color-item-bg-selected);
  }

  .path-list option:disabled {
    color: var(--color-input-placeholder);
    cursor: default;
  }

  .button-group {
    display: flex;
    justify-content: flex-end;
    gap: 1rem;
  }

  .primary-button, .secondary-button {
    padding: 0.6rem 1.5rem;
    font-family: inherit;
    font-size: 1em;
    border: 0;
    border-radius: 2rem;
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .progress-bar-container {
    display: flex;
    align-items: center;
  }

  .progress-bar {
    flex: 1;
    height: 1.5rem;
    background-color: var(--color-input-bg);
    border: 1px solid var(--color-input-border);
    border-radius: 0.5rem;
    overflow: hidden;
    position: relative;
  }

  .progress-bar-fill {
    height: 100%;
    background-color: var(--color-button-primary-bg);
    transition: width 0.15s ease;
  }

  .progress-text {
    margin-right: 1rem;
    font-family: monospace;
    font-size: 1em;
    color: var(--color-text);
    white-space: nowrap;
    min-width: 4rem;
    text-align: right;
  }

  .log-display {
    width: 100%;
    height: 15rem;
    padding: 1rem;
    font-family: monospace;
    font-size: 0.9em;
    border: 1px solid var(--color-input-border);
    background-color: var(--color-input-bg);
    color: var(--color-text);
    border-radius: 0.5rem;
    resize: none;
    overflow-y: auto;
    box-sizing: border-box;
  }
</style>
