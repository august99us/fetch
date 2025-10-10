<script lang="ts">
  interface Props {
    query?: string;
    disabled?: boolean;
    onsearch?: (query: string) => void;
  }

  let {
    query = $bindable(''),
    disabled = false,
    onsearch
  }: Props = $props();

  function handleSubmit(event: Event) {
    event.preventDefault();
    onsearch?.(query);
  }
</script>

<form class="search-bar" onsubmit={handleSubmit}>
  <input
    type="text"
    class="search-input"
    placeholder="Enter query here..."
    bind:value={query}
    disabled={disabled}
  />
  <button
    type="submit"
    class="search-button"
    disabled={disabled}
  >
    {disabled ? 'Searching...' : 'Search'}
  </button>
</form>

<style>
  .search-bar {
    display: flex;
    gap: 0.3125em;
    width: 100%;
  }

  .search-input {
    flex: 1;
    padding: 0.3125em;
    font-family: inherit;
    font-size: 1em;
    border: 1px solid var(--color-input-border);
    background-color: var(--color-input-bg);
    color: var(--color-text);
    border-radius: 0.25em;
    outline: none;
    transition: all 0.15s ease;
  }

  .search-input:focus {
    border-color: var(--color-input-border-focus);
    background-color: var(--color-input-bg-focus);
  }

  .search-input::placeholder {
    color: var(--color-placeholder);
  }

  .search-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .search-button {
    padding: 0.3125em 1em;
    font-family: inherit;
    font-size: 1em;
    border: 1px solid var(--color-input-border);
    background-color: var(--color-button-bg);
    color: var(--color-text);
    border-radius: 0.25em;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .search-button:hover:not(:disabled) {
    background-color: var(--color-button-bg-hover);
  }

  .search-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
