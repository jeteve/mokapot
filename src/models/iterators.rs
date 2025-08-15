use crate::models::index::DocId;

/**
 * An docIDs iterator to iterate over a disjunction of other iterators of the SAME type.
 */
pub(crate) struct DisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    iterators: Vec<T>,
    seen: Vec<DocId>,
    current_docids: Vec<DocId>,
}

impl<T> DisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    pub(crate) fn new(iterators: Vec<T>) -> Self {
        let n_its = iterators.len();
        let docids = Vec::with_capacity(n_its);
        DisjunctionIterator {
            iterators,
            seen: Vec::with_capacity(n_its * 2),
            current_docids: docids,
        }
    }

    fn _advance_all_iters(&mut self) {
        // Advance all iterators and push the docIds in the current_docids.
        for it in self.iterators.iter_mut() {
            if let Some(doc_id) = it.next() {
                self.current_docids.push(doc_id);
            }
        }
    }
}

impl<T> Iterator for DisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    type Item = DocId;

    fn next(&mut self) -> Option<Self::Item> {
        // More or less the same algorithm as the general DisjunctionIterator,
        // but here theres is no ordering guarantee.
        // optimisation for queries containing just one term query.
        if self.iterators.len() == 1 {
            return self.iterators[0].next();
        }

        loop {
            // This would be made empty at the beginning
            // or when the current_docids buffer has been poped enough.
            if self.current_docids.is_empty() {
                self._advance_all_iters();

                if self.current_docids.is_empty() {
                    // still empty..
                    return None;
                }
            }

            // Ok, current_docids is NOT empty.
            // Otherwise we would have returned just above.
            let candidate = self.current_docids.pop().unwrap();
            if !self.seen.contains(&candidate) {
                self.seen.push(candidate);
                return Some(candidate);
            }
        }
    }
}
