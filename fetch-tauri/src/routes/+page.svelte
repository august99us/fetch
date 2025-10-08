<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";

  let query = $state("");
  let loading = $state(false);
  let results = $state<QueryResult[]>([]);
  let selectedIndex = $state(-1);
  let shifted = $state(false);

  let timeoutQuery: string | undefined;
  let timeoutId: number | undefined;
  function queryChanged() {
    if (timeoutQuery !== query) {
      // any selections are no longer valid
      selectedIndex = -1;

      // query text is not the query we are running right now
      timeoutQuery = query;
      if (timeoutId) {
        clearTimeout(timeoutId);
        timeoutId = undefined;
      }

      // if we have a non-empty query, start a search
      if (query && query !== "") {
        loading = true;
        timeoutId = setTimeout(() => {
          fireQuery(timeoutQuery!);
          timeoutId = undefined;
        }, 500);
      } else {
        // cancel any existing searches
        //
        // This is done right now by noting down the query that the timeout was fired for,
        // and ignoring any searches return with a query no longer matching that query.
        // This will result in extra searches being performed for now.
      }
    }
  }

  function progress() {
    console.log("progress called");
    if (selectedIndex >= 0 && selectedIndex < results.length) {
      openIndex(selectedIndex);
    } else {
      goto("/full");
    }
  }

  function openIndex(index: number) {
    const result = results[index];
    if (shifted) {
      // open location
      console.log("Opening result location: " + result);
      invoke("open_location", { path: result.path })
        .then(() => {
          console.log("Opened result location: " + result);
        })
        .catch((e) => {
          console.error("Error opening for result location: " + e);
        });
      return;
    } else {
      console.log("Opening result: " + result);
      invoke("open", { path: result.path })
        .then(() => {
          console.log("Opened result: " + result);
        })
        .catch((e) => {
          console.error("Error opening result: " + e);
        });
    }
  }

  function fireQuery(query: string) {
    console.log("Searching for: " + query);
    queryIndex(query)
      .then(([q, res]) => {
        console.log("Got results for query: " + q + " : " + res);
        if (q === timeoutQuery) {
          results = res;
          loading = false;
          selectedIndex = 0;
        } else {
          console.log("Ignoring results for old query: " + q);
        }
      })
      .catch((e) => {
        console.error("Error querying index: ", e);
        loading = false;
      });
  }

  interface QueryResult {
    name: string;
    path: string;
    score: number;
  }

  async function queryIndex(query: string): Promise<[string, QueryResult[]]> {
    return [query, await invoke("query", { query, page: 1 })];
  }

  function parseResultName(result: QueryResult): string {
    return result.name;
  }

  function parseResultDescriptor(result: QueryResult): string {
    return result.path + " (score: " + result.score.toFixed(2) + ")";
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Shift') {
      shifted = true;
    }

    if (results.length !== 0) {
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
      } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
      }
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (event.key === 'Shift') {
      shifted = false;
    }
  }

  function handleResultMouseEnter(index: number) {
    selectedIndex = index;
  }

  function handleResultClick(index: number) {
    selectedIndex = index;
    openIndex(selectedIndex);
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  });
</script>

<main class="container">
  <div id="search-container">
    <form id="search-form" onsubmit={progress}>
      <span id="search-bar">
        <input id="search-input" type="text" placeholder="Start typing to search or press enter to open full app." bind:value={query} oninput={queryChanged} />
        <button type="submit" class="logo-button">
          <img src="/fetch.svg" class="logo" alt="Fetch Logo" />
        </button>
      </span>
    </form>
    {#if loading||(results.length > 0)}
      <div id="results-container">
        {#if loading}
          <div class="spinner" in:fade></div>
        {/if}
        {#if loading}
          <div class="spinner-placeholder"></div>
        {:else}
          {#each results as result, index}
            <button
              class="result-item"
              class:selected={index === selectedIndex}
              transition:fade
              onmouseenter={() => handleResultMouseEnter(index)}
              onclick={() => handleResultClick(index)}
            >
              <span class="result-name">{parseResultName(result)}</span>
              <span class="result-descriptor">{parseResultDescriptor(result)}</span>
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</main>

<style>
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 300;
  src: url('/fonts/Inter/Inter_24pt-Light.ttf') format('truetype');
}

@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 400;
  src: url('/fonts/Inter/Inter_24pt-Regular.ttf') format('truetype');
}

@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 500;
  src: url('/fonts/Inter/Inter_24pt-Medium.ttf') format('truetype');
}

@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 600;
  src: url('/fonts/Inter/Inter_24pt-SemiBold.ttf') format('truetype');
}

:root {
  /* Color variables */
  --color-text: #0f0f0f;
  --color-background: #f6f6f6;
  --color-section-bg: rgba(150, 150, 150, 0.1);
  --color-section-border: var(--color-background);
  --color-input-bg: rgba(255, 255, 255, 0.95);
  --color-input-bg-focus: rgba(255, 255, 255, 0.95);
  --color-input-border: rgba(0, 0, 0, 0.15);
  --color-placeholder: rgba(15, 15, 15, 0.4);
  --color-accent-glow: rgba(0, 0, 0, 0.3);
  --logo-opacity: 0.9;
  --backdrop-blur: 0px;

  /* Shadow variables */
  --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.08);
  --shadow-md: 0 8px 32px rgba(0, 0, 0, 0.1);
  --shadow-lg: 0 12px 40px rgba(0, 0, 0, 0.15);
  --shadow-focus: 0 4px 12px rgba(0, 0, 0, 0.1);

  /* Spinner variables */
  --color-spinner-bg: rgba(0, 0, 0, 0.1);
  --color-spinner-fg: rgba(0, 0, 0, 0.5);

  /* Result item variables */
  --color-result-selected-bg: rgba(0, 0, 0, 0.1);
  --color-result-descriptor: rgba(15, 15, 15, 0.5);

  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: var(--color-text);
  background-color: var(--color-background);

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

:global(body) {
  overflow: hidden;
}

.container {
  padding: 0 0.5rem;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  align-items: center;
  text-align: center;
  min-height: 100vh;
}

#search-container {
  width: 100%;
  max-width: 1200px;
  height: 100%;
  padding: 0.5rem;
  display: flex;
  justify-content: center;
  align-items: center;
  flex-direction: column;
  gap: 0.75rem;
  border: 1px solid var(--color-section-border);
  border-radius: 10px;
  background-color: var(--color-section-bg);
  transition: all 0.3s ease;
}

#search-form {
  width: 100%;
  display: flex;
  justify-content: center;
}

.logo-button {
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.logo {
  height: 4em;
  will-change: filter;
  transition: filter 0.3s ease;
  opacity: var(--logo-opacity);
}

.logo-button:hover {
  filter: drop-shadow(0 0 1em var(--color-accent-glow));
  opacity: 1;
}

#search-bar {
  display: flex;
  width: 100%;
  flex-direction: row;
  align-items: center;
  gap: 0.5rem;
}

#search-input {
  flex: 1;
  min-width: 0;
  padding: 0.75em 1.25em;
  font-size: 1.25rem;
  font-family: inherit;
  border: 1px solid var(--color-input-border);
  border-radius: 10px;
  background-color: var(--color-input-bg);
  box-shadow: var(--shadow-md), var(--shadow-sm);
  outline: none;
  transition: all 0.2s ease;
  color: var(--color-text);
  backdrop-filter: blur(var(--backdrop-blur));
}

#search-input:focus {
  background-color: var(--color-input-bg-focus);
  box-shadow: var(--shadow-lg), var(--shadow-focus);
}

#search-input::placeholder {
  color: var(--color-placeholder);
}

#results-container {
  position: relative;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.spinner {
  /* Center the spinner in container */
  position: absolute;
  left: 0;
  margin-left: auto;
  right: 0;
  margin-right: auto;
  top: 0;
  margin-top: auto;
  bottom: 0;
  margin-bottom: auto;
  width: fit-content;
  height: fit-content;
  /* Spinner styles */
  border: 4px solid var(--color-spinner-bg);
  border-top: 4px solid var(--color-spinner-fg);
  border-radius: 50%;
  width: 2rem;
  height: 2rem;
  animation: spin 1s linear infinite; /* Apply the spin animation */
  z-index: 9;
}

.spinner-placeholder {
  width: 2rem;
  height: 2rem;
  margin: 0.5rem;
}

@keyframes spin {
  0% {
    transform: rotate(0deg); /* Start at 0 degrees rotation */
  }
  100% {
    transform: rotate(360deg); /* End at 360 degrees rotation */
  }
}

.result-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 92%;
  padding: 0.5rem 0.75rem;
  border-radius: 5px;
  background-color: transparent;
  cursor: pointer;
  border: none;
  font-family: inherit;
  text-align: inherit;
}

.result-item.selected {
  background-color: var(--color-result-selected-bg);
}

.result-name {
  font-size: 1.25rem;
  color: var(--color-text);
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex-shrink: 1;
  flex-grow: 0;
  min-width: 5rem;
}

.result-descriptor {
  font-size: 0.9rem;
  color: var(--color-result-descriptor);
  text-align: right;
  margin-left: 1rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex-shrink: 0;
  flex-grow: 1;
  flex-basis: 3rem;
  min-width: 3rem;
}

@media (prefers-color-scheme: light) {
  :root {
    --color-text: #f6f6f6;
    --color-background: rgba(20, 20, 20, 0.95);
    --color-section-bg: rgba(45, 45, 45, 0.9);
    --color-section-border: rgba(170, 170, 170, 0.1);
    --color-input-bg: rgba(40, 40, 40, 0.8);
    --color-input-bg-focus: rgba(45, 45, 45, 0.9);
    --color-input-border: rgba(200, 200, 200, 0.15);
    --color-placeholder: rgba(246, 246, 246, 0.35);
    --color-accent-glow: rgba(50, 50, 50, 0.95);
    --logo-opacity: 0.85;
    --backdrop-blur: 20px;

    --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.2);
    --shadow-md: 0 8px 32px rgba(0, 0, 0, 0.3);
    --shadow-lg: 0 12px 40px rgba(0, 0, 0, 0.4);
    --shadow-focus: 0 4px 12px rgba(0, 0, 0, 0.25);
    
    --color-spinner-bg: rgba(255, 255, 255, 0.15);
    --color-spinner-fg: rgba(255, 255, 255, 0.3);

    --color-result-selected-bg: rgba(255, 255, 255, 0.1);
    --color-result-descriptor: rgba(246, 246, 246, 0.5);
  }
}
</style>
