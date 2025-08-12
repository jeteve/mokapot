use super::query::*;
use crate::itertools::TheShwartz;
use crate::models::documents::Document;
use crate::models::index::*;

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

    fn to_document_with_sample(&self, sample: &Index) -> Document {
        if self.queries.is_empty() {
            return Document::default();
        }
        dbg!(self
            .queries
            .iter()
            .schwartzian(
                |q| q.specificity_in_sample(sample),
                |sa, sb| sa.total_cmp(sb).reverse(),
            )
            .next()
            .unwrap())
        .to_document()
    }

    fn to_document(&self) -> Document {
        // This is the to_document of the most specific subquery.
        // ( (f1=a OR f2=b)[0.5] AND f2=c[1] )[1.5]
        // would choose f2=c
        if self.queries.is_empty() {
            return Document::default();
        }

        // Find the most specific subquery and to_document it.
        dbg!(self
            .queries
            .iter()
            .schwartzian(|q| q.specificity(), |sa, sb| sa.total_cmp(sb).reverse())
            .next()
            .unwrap()
            .to_document())
    }

    fn specificity(&self) -> f64 {
        // The sum of sub specifities
        // This is because a conjunction becomes more an more specific
        // as subqueries accumulate.
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
