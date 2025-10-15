<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Filtering from "$lib/components/search/Filtering.svelte";
  import SearchBar from "$lib/components/search/SearchBar.svelte";
  import ResultsArea from "$lib/components/search/ResultsArea.svelte";
  import Pagination from "$lib/components/search/Pagination.svelte";
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
  async function handleSearch(searchQuery: string, page: number = 1) {
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

  async function handleChangePage(newPage: number) {
    handleSearch(query, newPage);
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
  <SearchBar
    bind:query
    disabled={loading}
    onsearch={handleSearch}
  />

  <Filtering />

  <div class="results-container">
    {#if loading}
      <div class="spinner centered-above"></div>
    {/if}
    <ResultsArea
      bind:this={resultsArea}
      {results}
      {selectedIndex}
      disabled={loading}
      onselect={handleSelectResult}
      onopen={handleOpenFile}
    />
  </div>

  <Pagination
    bind:currentPage
    disabled={results.length == 0 || loading}
    onchangepage={handleChangePage}
  />
</main>

<style>
  :global(body, html) {
    margin: 0;
    padding: 0;
  }

  .full-search {
    width: 100%;
    height: 100vh;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    background-color: var(--color-background);
    color: var(--color-text);
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    overflow: hidden;
    padding: 1.5rem 1.5rem 1.1rem 1.5rem;
    gap: 1rem;
  }

  .results-container {
    position: relative;
    display: flex;
    flex: 1 1 0;
    min-height: 0;
  }

  .centered-above {
    /* Center the element in container and place above everything */
    position: absolute;
    margin: auto;
    inset: 0;
    z-index: 99;
  }
</style>
