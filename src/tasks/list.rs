#[derive(thiserror::Error, Debug)]
pub enum ListError {
    #[error("Unable to acquire lock on list")]
    Locking { #[source] source: anyhow::Error },
    #[error("Unknown or dependency error occurred")]
    Other { #[source] source: anyhow::Error },
}

/// Guarantees each operation is atomic and thread-safe
/// This includes iter_mut, which locks the entire list for the lifetime of the iterator
/// 
/// Why linked list?
/// 1. need O(n) anyway because the goal is to persist the data to disk (with changes)
/// 2. most of the operations will be around the beginning or at the very end of the list
/// 3. small memory footprint because operations will be streamable
/// 4. simpler to think about than some swapping file bytes/truncating file lenghts magic
pub trait SyncedLinkedList<I> {
    fn append(&mut self, item: I) -> Result<(), ListError>;
    fn iter_mut(&mut self) -> Result<impl Iterator<Item = Result<impl WrappedItem<I>, ListError>>, ListError>;
}

/// For use during iter_mut in the SyncedList trait
/// Provides mutator and accessor methods for the item in the iteration
pub trait WrappedItem<I> {
    fn get(&self) -> I;
    fn set(&mut self, item: I);
    fn remove(&mut self);
}

pub mod file_based_linked_list;