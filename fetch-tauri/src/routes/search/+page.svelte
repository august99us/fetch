<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import ResultsArea from "$lib/components/ResultsArea.svelte";
  import Pagination from "$lib/components/Pagination.svelte";
  import "$lib/styles/colors.css";

  interface QueryResult {
    name: string;
    path: string;
    score: number;
  }

  interface FileResult {
    path: string;
    name: string;
    previewUri?: string;
  }

  let query = $state("");
  let currentPage = $state(1);
  let loading = $state(false);
  let results = $state<FileResult[]>([]);
  let selectedIndex = $state(-1);

  let resultsArea: ResultsArea;

  // TODO: Implement query execution
  async function handleSearch(searchQuery: string) {
    if (!searchQuery || searchQuery.trim() === "") {
      results = [];
      selectedIndex = -1;
      return;
    }

    loading = true;
    try {
      // Call the Tauri query command
      const queryResults: QueryResult[] = await invoke("query", {
        query: searchQuery,
        page: currentPage
      });

      // Transform results to FileResult format
      results = queryResults.map(result => ({
        path: result.path,
        name: result.name,
        previewUri: undefined // TODO: Implement preview loading
      }));

      selectedIndex = results.length > 0 ? 0 : -1;
    } catch (error) {
      console.error("Error querying index:", error);
      results = [];
      selectedIndex = -1;
    } finally {
      loading = false;
    }
  }

  // TODO: Implement pagination
  function handleNextPage() {
    currentPage++;
    handleSearch(query);
  }

  function handlePreviousPage() {
    if (currentPage > 1) {
      currentPage--;
      handleSearch(query);
    }
  }

  // TODO: Implement file opening
  function handleOpenFile(index: number, path: string) {
    console.log("Opening file:", path);
    invoke("open", { path })
      .then(() => console.log("Opened file:", path))
      .catch((e) => console.error("Error opening file:", e));
  }

  // TODO: Implement result selection
  function handleSelectResult(index: number) {
    selectedIndex = index;
  }

  // Keyboard navigation
  function handleKeyDown(event: KeyboardEvent) {
    if (results.length === 0) return;

    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        resultsArea?.handleArrowKey('left');
        break;
      case 'ArrowRight':
        event.preventDefault();
        resultsArea?.handleArrowKey('right');
        break;
      case 'ArrowUp':
        event.preventDefault();
        resultsArea?.handleArrowKey('up');
        break;
      case 'ArrowDown':
        event.preventDefault();
        resultsArea?.handleArrowKey('down');
        break;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<main class="full-search">
  <a href="/">back</a>
  <div class="container">
    <SearchBar
      bind:query
      disabled={loading}
      onsearch={handleSearch}
    />

    <div class="divider"></div>

    <div class="results-container">
      {#if loading}
        <div class="loading-overlay">
          <div class="spinner"></div>
          <p>Searching...</p>
        </div>
      {/if}
      <ResultsArea
        bind:this={resultsArea}
        {results}
        {selectedIndex}
        onselect={handleSelectResult}
        onopen={handleOpenFile}
      />
    </div>

    <Pagination
      {currentPage}
      disabled={loading}
      onprevious={handlePreviousPage}
      onnext={handleNextPage}
    />
  </div>
</main>

<style>
  .full-search {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background-color: var(--color-background);
    color: var(--color-text);
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    overflow: hidden;
  }

  .container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    padding: 0.3125em;
    gap: 0.3125em;
  }

  .divider {
    width: 100%;
    height: 1px;
    background-color: var(--color-section-border);
  }

  .results-container {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .loading-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background-color: rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(2px);
    z-index: 10;
    color: var(--color-text);
    gap: 1em;
  }

  .spinner {
    border: 4px solid var(--color-spinner-bg);
    border-top: 4px solid var(--color-spinner-fg);
    border-radius: 50%;
    width: 2.5em;
    height: 2.5em;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .loading-overlay p {
    font-size: 1.25em;
  }
</style>
