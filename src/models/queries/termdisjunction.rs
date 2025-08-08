use crate::models::documents::Document;
use crate::models::index::{DocId, Index};
use crate::models::queries::query::Query;
use crate::models::queries::TermQuery;

// A Specialised Term disjunction query
// for use by the Percolator.
#[derive(Debug)]
pub struct TermDisjunction {
    queries: Vec<TermQuery>,
}
impl TermDisjunction {
    pub fn new(queries: Vec<TermQuery>) -> Self {
        TermDisjunction { queries }
    }

    pub fn matches(&self, d: &Document) -> bool {
        self.queries.iter().any(|q| q.matches(d))
    }

    pub fn dids_from_idx<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + use<'a> {
        let iterators: Vec<_> = self
            .queries
            //.sort_by_cached_key(|q);
            .iter()
            .map(|q| q.dids_from_idx(index))
            .collect();
        TermDisjunctionIterator::new(iterators)
    }
}

// Can only hold a collection of ONE type of iterator.
struct TermDisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    iterators: Vec<T>,
    seen: Vec<DocId>,
    current_docids: Vec<DocId>,
}

impl<T> TermDisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    fn new(iterators: Vec<T>) -> Self {
        let n_its = iterators.len();
        let docids = Vec::with_capacity(n_its);
        TermDisjunctionIterator {
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

impl<T> Iterator for TermDisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    type Item = DocId;

    fn next(&mut self) -> Option<Self::Item> {
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

/* impl Iterator for DisjunctionIterator<'_> {
    type Item = DocId;

    fn next(&mut self) -> Option<Self::Item> {
        // Ideas for the iterator:
        // Maintain a set of seen docIds
        loop {
            // This would be made empty at the beginning
            // or when the current_docids buffer has been poped enough.
            if self.current_docids.is_empty() {
                // If we have no current docids, we need to advance all iterators
                for it in self.iterators.iter_mut() {
                    if let Some(doc_id) = it.next() {
                        self.current_docids.push(doc_id);
                    }
                }

                if self.current_docids.is_empty() {
                    // If we have no current docids despite
                    // just having tried to populate, we are done for good.
                    return None;
                }

                // Current docids is Not empty

                // Go from lower to higher doc IDs.
                // It is important to preserve the order.
                self.current_docids.sort();
                self.current_docids.reverse();
                // Cleanup seen from anything lower than min
                let &new_min = self
                    .current_docids
                    .last()
                    .expect("current_docids should not be empty here");
                // Only keep stuff greater or equal than this new min
                // Note that if we have seen the new_min, we need to keep
                // is as seen. So we retain it too.
                self.seen.retain(|&e| e >= new_min);
            }

            // This will return all the matching doc IDs,
            // with potential duplicates.

            // current_docids is not empty here.
            let candidate = self
                .current_docids
                .pop()
                .expect("current_docids should not be empty");

            if !self.seen.contains(&candidate) {
                // Candidate has not been seen.
                // It is a good one!
                self.seen.insert(candidate);
                return Some(candidate);
            }
        }
    }
}

*/
