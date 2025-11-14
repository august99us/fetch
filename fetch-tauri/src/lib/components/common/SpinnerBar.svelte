<script>
    import { fade } from "svelte/transition";
</script>

<div class="spinner-placeholder">
    <div class="spinner" in:fade={{ duration: 200 }}></div>
</div>

<style>
  .spinner {
    /* Spinner styles */
    border: 4px solid var(--color-spinner-bg);
    border-top: 4px solid var(--color-spinner-fg);
    border-radius: 50%;
    width: 2rem;
    height: 2rem;
    animation: spin 1s linear infinite; /* Apply the spin animation */
  }

  .spinner-placeholder {
    height: 2rem;
    margin: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg); /* Start at 0 degrees rotation */
    }
    100% {
      transform: rotate(360deg); /* End at 360 degrees rotation */
    }
  }
</style>