<script lang="ts">
  interface Props {
    currentPage?: number;
    totalPages?: number;
    disabled?: boolean;
    onchangepage?: (page: number) => Promise<void>;
  }

  let {
    currentPage = $bindable(1),
    totalPages = 0,
    disabled = false,
    onchangepage,
  }: Props = $props();

  let pages = $derived.by<number[]>(() => {
    let startIndex = Math.max(1, currentPage - 2);
    let endIndex = totalPages > 0 ? Math.min(totalPages, currentPage + 2) : currentPage + 2;

    return Array.from({ length: endIndex - startIndex + 1 }, (_, i) => startIndex + i);
  });

  async function changePage(page: number) {
    if (!disabled && page !== currentPage) {
      await onchangepage?.(page);
      currentPage = page;
    }
  }

  async function handlePrevious() {
    if (!disabled && currentPage > 1) {
      let newPage = currentPage - 1;
      await changePage(newPage);
    }
  }

  async function handleNext() {
    if (!disabled && (currentPage < totalPages || totalPages === 0)) {
      let newPage = currentPage + 1;
      await changePage(newPage);
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
    <span class="arrow">&lt;</span>
  </button>

  {#if pages[0] > 1}
    <button
      class="page-button"
      disabled={disabled || currentPage === 1}
      onclick={() => changePage(1)}
      aria-label="Go to page 1"
    >
      1
    </button>
    {#if pages[0] > 2}
      <span class="ellipsis" class:disabled>...</span>
    {/if}
  {/if}

  {#each pages as page}
    <button
      class="page-button"
      class:current={page === currentPage}
      disabled={disabled || page === currentPage}
      onclick={() => changePage(page)}
      aria-label={"Go to page " + page}
    >
      {page}
    </button>
  {/each}

  {#if totalPages == 0 || pages[pages.length - 1] < totalPages - 1}
    <span class="ellipsis" class:disabled>...</span>
  {/if}
  {#if totalPages && pages[pages.length - 1] < totalPages}
      <button
        class="page-button"
        disabled={disabled || currentPage === totalPages}
        onclick={() => changePage(totalPages)}
        aria-label={"Go to page " + totalPages}
      >
        {totalPages}
      </button>
  {/if}

  <button
    class="page-button"
    disabled={disabled}
    onclick={handleNext}
    aria-label="Next page"
  >
    <span class="arrow">&gt;</span>
  </button>
</div>

<style>
  .pagination {
    display: flex;
    flex: 0 1 auto;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    width: 100%;
  }

  .page-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    padding: 0.25rem;
    font-family: inherit;
    border: 0;
    background-color: var(--color-background);
    color: var(--color-text);
    cursor: pointer;
  }

  .page-button:hover:not(:disabled) {
    background-color: var(--color-button-bg-secondary-hover);
  }

  .page-button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .ellipsis {
    color: var(--color-text);
    opacity: 0.4;
    cursor: default;
  }
</style>
