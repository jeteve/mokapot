use crate::models::index::DocId;

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

pub(crate) struct ConjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    iterators: Vec<T>,
    iterator_levels: Vec<Option<DocId>>,
    watermark: DocId,
}

impl<T> ConjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    pub(crate) fn new(iterators: Vec<T>) -> Self {
        let iterator_levels = vec![None; iterators.len()];
        ConjunctionIterator {
            iterators,
            iterator_levels,
            watermark: DocId::MAX,
        }
    }
}

impl<T> Iterator for ConjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    type Item = DocId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterators.is_empty() {
            return None;
        }

        loop {
            // Advance all iterators that are lower than the watermark
            for (idx, level) in self.iterator_levels.iter_mut().enumerate() {
                match level {
                    Some(doc_id) if *doc_id < self.watermark => {
                        let next = self.iterators[idx].next();
                        if let Some(next_docid) = next {
                            assert!(
                                next_docid >= *doc_id,
                                "Invariant broken: next_docid={} < doc_id={} for iterator {}",
                                next_docid,
                                doc_id,
                                idx
                            );
                        }
                        *level = next;
                    }
                    None => {
                        // We need to advance. We never advanced before
                        *level = self.iterators[idx].next();
                    }
                    _ => continue, // No need to advance
                }
            }
            // Any None means we are done.
            if self.iterator_levels.iter().any(|id| id.is_none()) {
                return None;
            }

            // None of the levels are None.

            // Next watermark and test for consistency match
            self.watermark = self
                .iterator_levels
                .iter()
                // unwrap is safe because of the invariant checked above
                .map(|id| id.unwrap())
                .max()
                .unwrap();

            // If all iterators have the same level, return the level.
            if self
                .iterator_levels
                .iter()
                .all(|&id| id == Some(self.watermark))
            {
                // Push the watermark up by for the next round, so all iterators are advanced
                self.watermark += 1;
                return self.iterator_levels[0];
            }

            // next iteration will advance the iterators that are late on the watermark.
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DocByIt {
    doc_id: DocId,
    it_index: usize,
}

impl std::cmp::PartialOrd for DocByIt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.doc_id.cmp(&other.doc_id))
    }
}
impl std::cmp::Ord for DocByIt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.doc_id.cmp(&other.doc_id)
    }
}

/**
 * An docIDs iterator to iterate over a disjunction of other iterators of the SAME type.
 * Given iterators MUST be ordered.
 */
pub(crate) struct DisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    iterators: Vec<T>,
    seen: HashSet<DocId>,
    candidates: BinaryHeap<Reverse<DocByIt>>,
}

impl<T> DisjunctionIterator<T>
where
    T: Iterator<Item = DocId>,
{
    pub(crate) fn new(iterators: Vec<T>) -> Self {
        let n_its = iterators.len();
        DisjunctionIterator {
            iterators,
            seen: HashSet::with_capacity(n_its * 2),
            candidates: BinaryHeap::with_capacity(n_its * 2),
        }
    }

    fn _fetch_all_iters(&mut self) {
        // Advance all iterators and push the docIds in the current_docids.
        for (it_index, it) in self.iterators.iter_mut().enumerate() {
            if let Some(doc_id) = it.next() {
                self.candidates.push(Reverse(DocByIt { doc_id, it_index }));
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
        // Guarantees order and uniquness of emitted docIds
        loop {
            if self.candidates.is_empty() {
                self._fetch_all_iters();
                if self.candidates.is_empty() {
                    // If still have no candidates, we are done.
                    return None;
                }
            }
            // Ok we have some candidates.

            // Now we can pop the smallest candidate.
            let candidate = self
                .candidates
                .pop()
                .expect("candidate is not empty here")
                .0;
            // For next time, we advance this smallest candidate's iterator
            if let Some(new_docid) = self.iterators[candidate.it_index].next() {
                // Enforce ordering invariant
                assert!(
                    new_docid >= candidate.doc_id,
                    "Invariant broken: new_docid={} from iterators[{}] < latest doc ID={}",
                    new_docid,
                    candidate.it_index,
                    candidate.doc_id
                );
                self.candidates.push(Reverse(DocByIt {
                    doc_id: new_docid,
                    it_index: candidate.it_index,
                }));
            }

            if !self.seen.contains(&candidate.doc_id) {
                // If we have not seen this candidate, we can return it.
                self.seen.insert(candidate.doc_id);
                // Cleanup. Do it when things become too big only.
                if self.seen.len() > 1024 {
                    self.seen.retain(|&id| id >= candidate.doc_id);
                }
                return Some(candidate.doc_id);
            }
            // If we have seen it, we just continue until there are no more candidates.
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_conjunction_iterator() {
        let mut ci: ConjunctionIterator<std::ops::Range<usize>> = ConjunctionIterator::new(vec![]);
        assert_eq!(ci.next(), None);
        assert_eq!(ci.next(), None);

        // Single iterator
        let mut ci = ConjunctionIterator::new(vec![(0..=3)]);
        assert_eq!(ci.next(), Some(0));
        assert_eq!(ci.next(), Some(1));
        assert_eq!(ci.next(), Some(2));
        assert_eq!(ci.next(), Some(3));
        assert_eq!(ci.next(), None);
        assert_eq!(ci.next(), None);

        // Overlapping
        let mut ci = ConjunctionIterator::new(vec![(0..=3), (2..=5)]);
        assert_eq!(ci.next(), Some(2));
        assert_eq!(ci.next(), Some(3));
        assert_eq!(ci.next(), None);
        assert_eq!(ci.next(), None);

        // Mixed
        let mut ci = ConjunctionIterator::new(vec![
            vec![0, 2, 3, 4, 6, 12, 13].into_iter(),
            vec![1, 3, 7, 12, 14].into_iter(),
            vec![1, 2, 3, 5, 8, 8, 9, 10, 11, 12].into_iter(),
        ]);
        assert_eq!(ci.next(), Some(3));
        assert_eq!(ci.next(), Some(12));
        assert_eq!(ci.next(), None);
        assert_eq!(ci.next(), None);
    }

    #[test]
    #[should_panic(expected = "Invariant broken: next_docid=2 < doc_id=4 for iterator 0")]
    fn test_broken_conjunction_iterator() {
        // Broken invariant:
        let ci = ConjunctionIterator::new(vec![
            vec![0, 2, 3, 4, 2, 6].into_iter(),
            vec![1, 3, 7].into_iter(),
            vec![1, 2, 9, 3, 5, 8, 8, 9].into_iter(),
        ]);
        let _ = ci.collect::<Vec<_>>();
    }

    #[test]
    fn test_disjunction_iterator() {
        let mut di: DisjunctionIterator<std::ops::Range<usize>> = DisjunctionIterator::new(vec![]);
        assert_eq!(di.next(), None);

        // Single iterator
        let mut di = DisjunctionIterator::new(vec![(0..=3)]);
        assert_eq!(di.next(), Some(0));
        assert_eq!(di.next(), Some(1));
        assert_eq!(di.next(), Some(2));
        assert_eq!(di.next(), Some(3));
        assert_eq!(di.next(), None);
        assert_eq!(di.next(), None);

        // Overlapping
        let mut di = DisjunctionIterator::new(vec![(0..=3), (2..=5)]);
        assert_eq!(di.next(), Some(0));
        assert_eq!(di.next(), Some(1));
        assert_eq!(di.next(), Some(2));
        assert_eq!(di.next(), Some(3));
        assert_eq!(di.next(), Some(4));
        assert_eq!(di.next(), Some(5));
        assert_eq!(di.next(), None);
        assert_eq!(di.next(), None);

        // Mixed
        let mut di = DisjunctionIterator::new(vec![
            vec![0, 2, 4, 6].into_iter(),
            vec![1, 3, 7].into_iter(),
            vec![1, 2, 3, 5, 8, 8, 9].into_iter(),
        ]);
        assert_eq!(di.next(), Some(0));
        assert_eq!(di.next(), Some(1));
        assert_eq!(di.next(), Some(2));
        assert_eq!(di.next(), Some(3));
        assert_eq!(di.next(), Some(4));
        assert_eq!(di.next(), Some(5));
        assert_eq!(di.next(), Some(6));
        assert_eq!(di.next(), Some(7));
        assert_eq!(di.next(), Some(8));
        assert_eq!(di.next(), Some(9));
        assert_eq!(di.next(), None);
        assert_eq!(di.next(), None);
    }

    #[test]
    #[should_panic(expected = "Invariant broken: new_docid=3 from iterators[2] < latest doc ID=9")]
    fn test_broken_invariant() {
        // Broken invariant:
        let di = DisjunctionIterator::new(vec![
            vec![0, 2, 4, 6].into_iter(),
            vec![1, 3, 7].into_iter(),
            vec![1, 2, 9, 3, 5, 8, 8, 9].into_iter(),
        ]);
        let _ = di.collect::<Vec<_>>();
    }
}
