import { json } from "@sveltejs/kit";
import { invoke } from "@tauri-apps/api/core";
import { untrack } from "svelte";

export interface ResolvedFileResult {
  rank: number;
  name: string;
  path: string;
  score: number;
}

// snake_case to match rust conventions
interface FileQueryingResult {
  results_len: number;
  changed_results: FileResult[];
  cursor_id: string | null;
}
interface FileResult {
  rank: number;
  old_rank: number | null;
  name: string;
  path: string;
  score: number;
}

export default class ReactiveBackgroundFetchQuery {
  query: string;
  resultsPerPage = $state<number>(20);
  page = $state<number>(1);
  querying = $state<boolean>(false);
  maxPages = $state<number | undefined>(undefined);
  hasMore = $state<boolean>(true);

  private cursorId = $state<string | null>("initial");
  private fullResultsList = $state.raw<ResolvedFileResult[]>([]);
  private windowedResultsList = $derived.by<ResolvedFileResult[]>(() => {
    const start = (this.page - 1) * this.resultsPerPage;
    const end = start + this.resultsPerPage;
    return this.fullResultsList.slice(start, end);
  });

  constructor(query: string, resultsPerPage: number = 20, page: number = 1) {
    this.query = query;
    this.resultsPerPage = resultsPerPage;
    this.page = page;
  }

  public nextPage() {
    // Can go to next page if we have more to fetch OR if we have cached results for the next page
    const nextPageNum = this.page + 1;
    const canGoNext = this.maxPages === undefined || nextPageNum <= this.maxPages;

    if (canGoNext) {
      this.page = nextPageNum;
    }
  }

  public previousPage() {
    if (this.page > 1) {
      this.page -= 1;
    }
  }

  public get results(): ResolvedFileResult[] {
    return this.windowedResultsList;
  }

  public get allResults(): ResolvedFileResult[] {
    return this.fullResultsList;
  }

  // You must call this function inside of an $effect in your component in order to
  // register the query's effects. You do not need to re-register the effect every
  // time the query changes, the component effect will automatically keep track of
  // that
  public effect() {
    $effect(() => {
      let totalRequiredResults = this.resultsPerPage * this.page;
      // The rest of the function must be untracked because it updates
      // state. We cannot use derived.by because it updates state more
      // than once while continually querying
      untrack(() => this.queryUntil(totalRequiredResults));
    })
  }

  private async queryUntil(numResults: number) {
    if (this.fullResultsList.length < numResults && this.hasMore) {
      this.querying = true;
      while (this.fullResultsList.length < numResults && this.hasMore) {
        try {
          console.log("querying");
          const result = await invoke<FileQueryingResult>("query", {
            query: this.query,
            cursorId: this.cursorId === "initial" ? null : this.cursorId,
          });

          // Merge changed results into full list
          this.processChangedResults(result.results_len, result.changed_results);

          // Update cursor
          this.cursorId = result.cursor_id;

          // If we've reached the end, set maxPages
          if (this.cursorId === null) {
            this.maxPages = Math.ceil(this.fullResultsList.length / this.resultsPerPage);
            this.hasMore = false;

            // Adjust page if it's beyond max
            if (this.page > this.maxPages) {
              this.page = Math.max(1, this.maxPages);
            }
          }
        } catch (error) {
          break;
        }
      }
      this.querying = false;
    }
  }

  private processChangedResults(newLen: number, changedResults: FileResult[]) {
    // I think without the ability to perform an atomic swap between two pieces of
    // memory (ie. i need to be able to swap listOne[i] and listTwo[j], assuming
    // both lists have the same object type), I will always need to copy the list
    // in order to perform replacements and moves. Otherwise, elements in the list
    // could be deallocated at random times, which may cause really weird behavior
    // like duplicated instances of the same result in multiple spots of the list
    let fullResultsListCopy = JSON.parse(JSON.stringify(this.fullResultsList));

    // Ensure array has sufficient capacity
    if (newLen > fullResultsListCopy.length) {
      fullResultsListCopy.length = newLen;
    }

    // Separate insertions and moves
    const insertions: FileResult[] = [];
    const moves = new Map<number, number>(); // old_rank -> new_rank

    for (const result of changedResults) {
      if (result.old_rank === null) {
        insertions.push(result);
      } else {
        moves.set(result.old_rank, result.rank);
      }
    }

    // Process insertions, then remaining moves
    for (const insertion of insertions) {
      placeItemAndResolveChain(fullResultsListCopy, insertion, moves);
    }

    while (moves.size > 0) {
      const [[oldRank, rank]] = moves;
      moves.delete(oldRank);
      let displacedItem = displacedItemFromResult(fullResultsListCopy[oldRank - 1], rank);
      // delete the item at the index, leaving an empty slot that returns
      // false for the (index in array) operation
      delete fullResultsListCopy[oldRank - 1];
      placeItemAndResolveChain(fullResultsListCopy, displacedItem, moves);
    }

    this.fullResultsList = fullResultsListCopy;
  }

}

function placeItemAndResolveChain(
  resultsList: ResolvedFileResult[],
  item: FileResult,
  moves: Map<number, number>
) {
  let current: FileResult | undefined = item;

  while (current) {
    const targetIndex = current.rank - 1;
    const displaced = resultsList[targetIndex];

    resultsList[targetIndex] = {
      rank: current.rank,
      name: current.name,
      path: current.path,
      score: current.score,
    };

    const nextRank = displaced && moves.get(displaced.rank);
    if (nextRank) {
      moves.delete(displaced.rank);
      current = displacedItemFromResult(displaced, nextRank)
    } else {
      current = undefined;
    }
  }
}

function displacedItemFromResult(displacedResult: ResolvedFileResult, newRank: number): FileResult {
  return {
    ...displacedResult,
    old_rank: displacedResult.rank,
    rank: newRank,
  }
}