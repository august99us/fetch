<script lang="ts">
  interface Props {
    currentPage?: number;
    disabled?: boolean;
    onprevious?: () => void;
    onnext?: () => void;
  }

  let {
    currentPage = 1,
    disabled = false,
    onprevious,
    onnext
  }: Props = $props();

  function handlePrevious() {
    if (!disabled && currentPage > 1) {
      onprevious?.();
    }
  }

  function handleNext() {
    if (!disabled) {
      onnext?.();
    }
  }
</script>

<div class="pagination">
  <button
    class="page-button"
    disabled={disabled || currentPage <= 1}
    onclick={handlePrevious}
    aria-label="Previous page"
  >
    <span class="arrow">◀</span>
  </button>

  <div class="page-info">
    <span>Page {currentPage}</span>
  </div>

  <button
    class="page-button"
    disabled={disabled}
    onclick={handleNext}
    aria-label="Next page"
  >
    <span class="arrow">▶</span>
  </button>
</div>

<style>
  .pagination {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5em;
    width: 100%;
    height: fit-content;
  }

  .page-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.6875em;
    height: 1.6875em;
    padding: 0.3125em;
    font-family: inherit;
    border: 1px solid var(--color-input-border);
    background-color: var(--color-button-bg-secondary);
    color: var(--color-text);
    border-radius: 0.25em;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .page-button:hover:not(:disabled) {
    background-color: var(--color-button-bg-secondary-hover);
  }

  .page-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .arrow {
    font-size: 0.875em;
    line-height: 1;
  }

  .page-info {
    font-size: 0.875em;
    color: var(--color-text);
    padding: 0 0.25em;
  }
</style>
