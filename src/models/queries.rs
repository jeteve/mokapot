use itertools::Itertools;

use crate::models::index::{DocId, Index};

use super::documents::Document;
use std::rc::Rc;

pub trait Query: std::fmt::Debug {
    /**
     * An iterator on all the DocIds matching this query in the index.
     */
    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a>;

    /**
     * An iterator on all the Documents matching this query in the index.
     */
    fn docs_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = &'a Document> + 'a> {
        Box::new(
            self.docids_from_index(index)
                .map(|doc_id| index.get_document(doc_id).expect("Document should exist")),
        )
    }

    /**
     * A Document for the purpose of indexing in the percolator
     */
    fn to_document(&self) -> Document;

    fn matches(&self, d: &Document) -> bool;

    /**
     * The a-priori specificity of a query, regardless of
     * any sample index.
     */
    fn specificity(&self) -> f64;
}

#[derive(Debug)]
pub struct ConjunctionQuery {
    queries: Vec<Box<dyn Query>>,
}
impl ConjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        ConjunctionQuery { queries }
    }
}

impl Query for ConjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.queries.iter().all(|q| q.matches(d))
    }

    fn to_document(&self) -> Document {
        // This is the to_document of the most specific subquery.
        // ( (f1=a OR f2=b)[0.5] AND f2=c[1] )[1.5]
        // would choose f2=c
        if self.queries.is_empty() {
            return Document::default();
        }

        // Find the most specific subquery and to_document it.
        self.queries
            .iter()
            .map(|q| (q, q.specificity()))
            .sorted_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(q, _)| q)
            .last()
            .unwrap()
            .to_document()
    }

    fn specificity(&self) -> f64 {
        // The sun of sub specifities
        self.queries.iter().map(|q| q.specificity()).sum()
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        // Get iterators from all queries
        let iterators: Vec<_> = self
            .queries
            .iter()
            .map(|q| q.docids_from_index(index))
            .collect();
        Box::new(ConjunctionIterator::new(iterators))
    }
}

struct ConjunctionIterator<'a> {
    iterators: Vec<Box<dyn Iterator<Item = DocId> + 'a>>,
    current_docids: Vec<Option<DocId>>,
    max_id: usize,
    last_match: Option<DocId>,
}
impl<'a> ConjunctionIterator<'a> {
    fn new(iterators: Vec<Box<dyn Iterator<Item = DocId> + 'a>>) -> Self {
        let n_its = iterators.len();
        ConjunctionIterator {
            iterators,
            current_docids: vec![None; n_its],
            max_id: usize::MIN,
            last_match: None,
        }
    }
}

impl Iterator for ConjunctionIterator<'_> {
    type Item = DocId;

    fn next(&mut self) -> Option<Self::Item> {
        // We need to advance all iterators that are lower than the current max_id
        let mut max_iterations = u32::MAX;
        loop {
            max_iterations -= 1;
            if max_iterations == 0 {
                panic!("Infinite loop detected in ConjunctionIterator");
            }
            let to_advance: Vec<usize> = self
                .current_docids
                .iter()
                .enumerate()
                .filter(|(_i, id)| {
                    match (id, self.last_match) {
                        (_, Some(_)) => true, // Always advance all if there was a match.
                        (Some(id), _) => {
                            // If we have a current ID, we only advance if it is lower than the max_id
                            *id < self.max_id
                        }
                        (None, _) => true, // Always advance none.
                    }
                })
                .map(|(i, _)| i)
                .collect(); // We only care about i.

            //println!("Advancing iterators: {:?}", to_advance);
            //return None;

            for i in to_advance {
                // Change doc ids in place.
                self.current_docids[i] = self.iterators[i].next();
            }

            //println!("Current docids: {:?}", self.current_docids);
            //return None;

            // If any is None, we are done, as there is no chance
            // to find a common document ID later on.
            if self.current_docids.iter().any(|id| id.is_none()) {
                // TODO later: Maybe mark this iterator as exhausted
                // So we dont do all the above ever again.
                //println!("Some iterators are exhausted, returning None");
                return None;
            } else {
                //println!("None of the iterators is exhausted, continuing");
            }

            //return None;

            self.max_id = self
                .current_docids
                .iter()
                .filter_map(|id| *id)
                // filter_map makes sure only
                // the Some values make it through, and they are unwrapped.
                // Hence the Iterator<Item = usize> type
                .max() // Vanilla max. on an iterator of usize
                .unwrap_or(usize::MIN);

            //println!("Max ID: {}", self.max_id);
            //return None;

            // If all current docids are the same, we return it
            if self
                .current_docids
                .iter()
                .all(|id| id == &Some(self.max_id))
            {
                //println!("All current docids are the same max, returning max_id");
                self.last_match = Some(self.max_id);
                return self.last_match;
            }

            // We havent returned anything, loop again
            // until we find a common document ID or
            // any of the iterator is exhausted.
            self.last_match = None; // No match. Next iteration will try
                                    // to advance up to the max_id
        }
    }
}

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
                // Go from lower to higher doc IDs.
                // It is important to preserve the order.
                if self.current_docids.is_empty() {
                    // If we have no current docids despite
                    // just having tried to populate, we are done for good.
                    return None;
                }

                // Current docids is Not empty

                self.current_docids.sort();
                self.current_docids.reverse();
                // Cleanup seen from anything lower than min
                let &new_min = self
                    .current_docids
                    .last()
                    .expect("current_docids should not be empty here");
                // Only keep stuff greater than this new min
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

#[derive(Debug, Clone)]
pub struct TermQuery {
    field: Rc<str>,
    term: Rc<str>,
}

impl TermQuery {
    pub fn new(field: Rc<str>, term: Rc<str>) -> Self {
        TermQuery { field, term }
    }
}

impl Query for TermQuery {
    fn matches(&self, d: &Document) -> bool {
        d.field_values_iter(&self.field)
            .map_or(false, |mut iter| iter.any(|value| value == &self.term))
    }
    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        Box::new(index.term_iter(self.field.clone(), self.term.clone()))
    }

    fn to_document(&self) -> Document {
        Document::default().with_value(self.field.clone(), self.term.clone())
    }

    fn specificity(&self) -> f64 {
        1.0
    }
}
