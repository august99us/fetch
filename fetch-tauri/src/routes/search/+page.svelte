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
  }

  let query = $state("");
  let currentPage = $state(1);
  let loading = $state(false);
  let resultsPromise = $state<Promise<FileResult[]> | undefined>(undefined);
  let resultsArea: ResultsArea | undefined = $state();

  // TODO: Implement query execution
  async function handleSearch(searchQuery: string, page: number = 1) {
    if (!searchQuery || searchQuery.trim() === "") {
      resultsPromise = undefined;
      return;
    }

    loading = true;
    try {
      // Call the Tauri query command
      resultsPromise = invoke<QueryResult[]>("query", {
        query: searchQuery,
        page
      })
      // Transform results to FileResult format
      .then((resArray: QueryResult[]) => 
        resArray.map((value: QueryResult): FileResult => { 
          return { path: value.path, name: value.name };
        })
      );
    } catch (error) {
      console.error("Error querying index:", error);
      resultsPromise = undefined;
    } finally {
      loading = false;
    }
  }

  async function handleChangePage(newPage: number) {
    console.log("Changing to page:", newPage);
    await handleSearch(query, newPage);
  }

  // TODO: Implement file opening
  function handleOpenFile(index: number, path: string) {
    console.log("Opening file:", path);
    invoke("open", { path })
      .then(() => console.log("Opened file:", path))
      .catch((e) => console.error("Error opening file:", e));
  }

  // Keyboard navigation
  function handleKeyDown(event: KeyboardEvent) {
    if (!resultsArea) return;

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
    {#if resultsPromise}
      {#await resultsPromise}
        <div class="spinner centered-above"></div>
        <ResultsArea
          bind:this={resultsArea}
          results = {[]}
          disabled={loading}
        />
      {:then results}
        <ResultsArea
          bind:this={resultsArea}
          {results}
          disabled={loading}
          onopen={handleOpenFile}
        />
      {:catch error}
        <div>
          <p>Error loading results</p>
        </div>
      {/await}
    {:else}
      <div class="empty-state" class:disabled={loading}>
        <p>Nothing yet!</p>
      </div>
    {/if}
  </div>

  <Pagination
    bind:currentPage
    disabled={!resultsArea || loading}
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

  .empty-state {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: top;
    justify-content: center;
    color: var(--color-input-placeholder);
    font-size: 1.1em;
  }

  .empty-state.disabled {
    opacity: 0.6;
    filter: grayscale(100%);
  }

  .centered-above {
    /* Center the element in container and place above everything */
    position: absolute;
    margin: auto;
    inset: 0;
    z-index: 99;
  }
</style>
