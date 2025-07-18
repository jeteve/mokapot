use super::query::*;
use crate::models::documents::Document;
use crate::models::index::{DocId, Index};

#[derive(Debug)]
pub struct DisjunctionQuery {
    queries: Vec<Box<dyn Query>>,
}
impl DisjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        DisjunctionQuery { queries }
    }
}
impl Query for DisjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.queries.iter().any(|q| q.matches(d))
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        let iterators: Vec<_> = self
            .queries
            //.sort_by_cached_key(|q);
            .iter()
            .map(|q| q.docids_from_index(index))
            .collect();
        Box::new(DisjunctionIterator::new(iterators))
    }

    fn to_document(&self) -> Document {
        // Returns all fields from the queries,
        // as the incoming document will turn into a Disjunction query.
        self.queries
            .iter()
            .fold(Document::default(), |a, q| a.merge_with(&q.to_document()))
    }

    fn specificity(&self) -> f64 {
        // Guarantee No NAN.
        if self.queries.is_empty() {
            0.0
        } else {
            // Just the Max of the sub specificities (or zero if nothing)
            self.queries
                .iter()
                .map(|q| q.specificity())
                .fold(f64::NAN, f64::max)
                / self.queries.len() as f64
        }
    }
}

struct DisjunctionIterator<'a> {
    iterators: Vec<Box<dyn Iterator<Item = DocId> + 'a>>,
    seen: std::collections::HashSet<DocId>,
    current_docids: Vec<DocId>,
}

impl<'a> DisjunctionIterator<'a> {
    fn new(iterators: Vec<Box<dyn Iterator<Item = DocId> + 'a>>) -> Self {
        let n_its = iterators.len();
        let docids = Vec::with_capacity(n_its);
        DisjunctionIterator {
            iterators,
            seen: std::collections::HashSet::with_capacity(n_its * 2),
            current_docids: docids,
        }
    }
}

impl Iterator for DisjunctionIterator<'_> {
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
