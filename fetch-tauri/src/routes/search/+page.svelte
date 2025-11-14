<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Filtering from "$lib/components/search/Filtering.svelte";
  import SearchBar from "$lib/components/search/SearchBar.svelte";
  import ResultsArea from "$lib/components/search/ResultsArea.svelte";
  import Pagination from "$lib/components/search/Pagination.svelte";
  import IndexDrawer from "$lib/components/index/IndexDrawer.svelte";
  import ReactiveBackgroundFetchQuery from "$lib/structs/ReactiveBackgroundFetchQuery.svelte";
  import "$lib/styles/colors.css";

  interface FileResult {
    path: string;
    name: string;
  }

  let query = $state("");
  let fetchQuery = $state<ReactiveBackgroundFetchQuery | undefined>(undefined);
  let resultsArea: ResultsArea | undefined = $state();

  // Derived state
  let results = $derived<FileResult[]>(
    (fetchQuery?.results ?? []).map(r => ({ path: r.path, name: r.name }))
  );
  let loading = $derived(fetchQuery?.querying ?? false);

  async function handleSearch(searchQuery: string) {
    if (!searchQuery || searchQuery.trim() === "") {
      fetchQuery = undefined;
      return;
    }

    console.log("Creating new query for:", searchQuery);
    fetchQuery = new ReactiveBackgroundFetchQuery(searchQuery, 9);
  }

  async function handleChangePage(newPage: number) {
    console.log("Changing to page:", newPage);
    if (fetchQuery) {
      fetchQuery.page = newPage;
    }
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

  $effect(() => {
    fetchQuery?.effect();
  })
</script>

<main class="full-search">
  <SearchBar
    bind:query
    disabled={loading}
    onsearch={handleSearch}
  />

  <Filtering />

  <div class="results-container">
    {#if fetchQuery}
      <ResultsArea
        bind:this={resultsArea}
        {results}
        loading={loading}
        onopen={handleOpenFile}
      />
    {:else}
      <div class="empty-state">
        <p>Nothing yet!</p>
      </div>
    {/if}
  </div>

  <Pagination
    currentPage={fetchQuery?.page ?? 1}
    totalPages={fetchQuery?.maxPages}
    disabled={!resultsArea || loading}
    onchangepage={handleChangePage}
  />

  <IndexDrawer />
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

    user-select: none;
    -webkit-user-select: none; /* For Safari */
    -moz-user-select: none;    /* For Firefox */
    -ms-user-select: none;     /* For Internet Explorer/Edge */
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
    padding-top: 5rem;
    color: var(--color-input-placeholder);
    font-size: 1.1em;
  }
</style>
