<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  let query = $state("");
  let loading = $state(false);
  let results = $state<QueryResult[]>([]);
  let selectedIndex = $state(-1);
  let shifted = $state(false);

  // Meta actions for the entire page ///////////////////////////////
  async function progress() {
    if (selectedIndex >= 0 && selectedIndex < results.length) {
      await openIndex(selectedIndex);
    } else {
      await openFull();
    }
    setTimeout(() => {
      closeCurrent();
    }, 50);
  }

  async function openFull() {
    let fullWindow = await WebviewWindow.getByLabel("full");
    if (!fullWindow) {
      // Full window doesn't exist. create it.
      fullWindow = new WebviewWindow('full', {
        url: "/search",
        title: "Fetch",
        width: 1200,
        height: 900,
        center: true,
      });
      await fullWindow.once('tauri://created', function () {
        console.log("Full window created");
      })
    }

    fullWindow.show();
  }

  async function openIndex(index: number) {
    const result = results[index];
    if (shifted) {
      // open location
      console.log("Opening result location: " + result);
      try {
        await invoke("open_location", { path: result.path });
        console.log("Opened result location: " + result);
      } catch (e) {
        console.error("Error opening for result location: " + e);
      }
    } else {
      console.log("Opening result: " + result);
      try {
        await invoke("open", { path: result.path });
        console.log("Opened result: " + result);
      } catch (e) {
        console.error("Error opening result: " + e);
      }
    }
  }

  async function closeCurrent() {
    const currentWindow = getCurrentWindow();
    await currentWindow.close();
  }

  // Query change tracking and handling ///////////////////////////////
  let timeoutQuery: string | undefined;
  let timeoutId: number | undefined;
  function queryChanged() {
    if (timeoutQuery !== query) {
      // any selections and results are no longer valid
      selectedIndex = -1;
      results = [];

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
        loading = false;
        // cancel any existing searches
        //
        // This is done right now by noting down the query that the timeout was fired for,
        // and ignoring any searches return with a query no longer matching that query.
        // This will result in extra searches being performed for now.
      }
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

  // JSX content functions and page utilities //////////////////////////////
  function parseResultName(result: QueryResult): string {
    return result.name;
  }

  function parseResultDescriptor(result: QueryResult): string {
    return result.path + " (score: " + result.score.toFixed(2) + ")";
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Shift') {
      shifted = true;
      return;
    }

    if (event.key === 'Escape') {
      closeCurrent();
      return;
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

  let ignoreFirstAfterResize = true;
  let lastMousePositionResult = { x: 0, y: 0 };
  function handleResultMouseOver(event: MouseEvent, index: number) {
    if (ignoreFirstAfterResize) {
      ignoreFirstAfterResize = false;
      return;
    }
    // Only update selection if the mouse actually moved
    if (event.clientX !== lastMousePositionResult.x || event.clientY !== lastMousePositionResult.y) {
      lastMousePositionResult = { x: event.clientX, y: event.clientY };
      selectedIndex = index;
    }
  }

  function handleWindowMouseMove(event: MouseEvent) {
    // Update the mouse position used to determine whether the mouse actually moved or not
    // for result focusing
    lastMousePositionResult = { x: event.clientX, y: event.clientY };
  }

  function handleResultFocus(index: number) {
    selectedIndex = index;
  }

  function handleResultClick(index: number) {
    selectedIndex = index;
    openIndex(selectedIndex);
  }

  let mainContainer: HTMLElement;
  async function resizeWindowToContent() {
    try {
      const appWindow = getCurrentWindow();
      const bodyWidth = mainContainer.offsetWidth;
      const bodyHeight = mainContainer.offsetHeight;
      console.log(`Resizing window to content: ${bodyWidth}x${bodyHeight}`);

      await appWindow.setSize(new LogicalSize(bodyWidth, bodyHeight));
      ignoreFirstAfterResize = true;
      await appWindow.center();
    } catch (error) {
      console.error("Error resizing and centering window:", error);
    }
  }

  function setMaxHeight() {
    if (mainContainer) {
      const maxHeight = window.screen.availHeight * 0.7;
      mainContainer.style.maxHeight = `${maxHeight}px`;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    // Set max height and resize window initially
    setMaxHeight();
    resizeWindowToContent();

    // Use ResizeObserver to watch for actual size changes
    const resizeObserver = new ResizeObserver(() => {
      resizeWindowToContent();
    });

    if (mainContainer) {
      resizeObserver.observe(mainContainer);
    }

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      resizeObserver.disconnect();
    };
  });

  // Scroll selected item into view
  let resultElements: HTMLButtonElement[] = [];
  $effect(() => {
    if (selectedIndex >= 0 && selectedIndex < resultElements.length) {
      resultElements[selectedIndex]?.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest'
      });
    }
  });
</script>

<svelte:window onmousemove={handleWindowMouseMove} />

<main class="container" bind:this={mainContainer}>
  <div id="search-container" data-tauri-drag-region>
    <form id="search-form" onsubmit={progress}>
      <span id="search-bar">
        <!-- svelte-ignore a11y_autofocus // element is not conditionally loaded -->
        <input
          id="search-input"
          type="text"
          placeholder="Start typing to search or press enter to open full app."
          bind:value={query}
          oninput={queryChanged}
          autocomplete="off"
          autofocus
        />
        <button type="submit" class="logo-button">
          <img src="/fetch.svg" class="logo" alt="Fetch Logo" />
        </button>
      </span>
    </form>
    {#if loading||(results.length > 0)}
      <div id="results-container" class:loading>
        {#if loading}
          <div class="spinner centered-above" in:fade={{ duration: 200 }}></div>
          <div class="spinner-placeholder"></div>
        {:else}
          {#each results as result, index}
            <button
              bind:this={resultElements[index]}
              class="result-item"
              class:selected={index === selectedIndex}
              transition:fade
              onmouseover={(e) => handleResultMouseOver(e, index)}
              onfocus={() => handleResultFocus(index)}
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
:root {
  /* Color variables */
  --backdrop-blur: 0px;

  color: var(--color-text);
  background-color: rgba(0, 0, 0, 0);
}

:global(body) {
  margin: 0;
  padding: 0;
  overflow: hidden;
}

.container {
  display: flex;
  flex-direction: column;
  justify-content: top;
  align-items: center;
  text-align: center;
  width: 100vw;
  min-width: 100vw;
  max-width: 100vw;
  margin: 0;
  border: 0;
  border-radius: 0.5rem;
  box-sizing: border-box;

  user-select: none;
  -webkit-user-select: none; /* For Safari */
  -moz-user-select: none;    /* For Firefox */
  -ms-user-select: none;     /* For Internet Explorer/Edge */
}

#search-container {
  width: 100%;
  min-width: 0;
  max-width: 1200px;
  max-height: 100%;
  padding: 0.5rem;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 0.75rem;
  flex-shrink: 0;
  border: 0;
  border-radius: 0.5rem;
  background-color: var(--color-background);
  transition: all 0.3s ease;
  box-sizing: border-box;
  user-select: none;
  overflow: hidden;
}

#search-form {
  width: 100%;
  display: flex;
  justify-content: center;
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
  padding: 0.75rem 1.25rem;
  font-size: 1.25rem;
  font-family: inherit;
  border: 1px solid var(--color-input-border);
  border-radius: 2rem;
  background-color: var(--color-input-bg);
  outline: none;
  transition: all 0.2s ease;
  color: var(--color-text);
  backdrop-filter: blur(var(--backdrop-blur));
}

#search-input:focus {
  background-color: var(--color-input-bg-focus);
}

#search-input::placeholder {
  color: var(--color-input-placeholder);
}

#results-container {
  position: relative;
  width: 100%;
  flex: 1 1 auto;
  min-height: 0;
  max-height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  overflow-y: auto;
  overflow-x: hidden;
}

#results-container.loading {
  pointer-events: none;
  overflow-y: hidden;
}

.centered-above {
  /* Center the element in container and place above everything */
  position: absolute;
  margin: auto;
  inset: 0;
  z-index: 99;
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
  background-color: var(--color-item-bg-selected);
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
  color: var(--color-item-descriptor);
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

@media (prefers-color-scheme: dark) {
  :root {
    --backdrop-blur: 20px;
  }
}
</style>
