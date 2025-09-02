use num_traits::{Bounded, One};
use std::ops::AddAssign;

pub(crate) struct ConjunctionIterator<T, I>
where
    T: Iterator<Item = I>,
    I: Bounded + Copy + AddAssign<I>,
{
    iterators: Vec<T>,
    iterator_levels: Vec<I>,
    is_init: bool,
    watermark: I,
}

impl<T, I> ConjunctionIterator<T, I>
where
    T: Iterator<Item = I>,
    I: Bounded + Copy + AddAssign<I>,
{
    pub(crate) fn new(iterators: Vec<T>) -> Self {
        let iterator_levels = vec![I::max_value(); iterators.len()];
        ConjunctionIterator {
            iterators,
            iterator_levels,
            is_init: false,
            watermark: I::max_value(),
        }
    }
}

impl<T, I> Iterator for ConjunctionIterator<T, I>
where
    T: Iterator<Item = I>,
    I: One + Bounded + Copy + Ord + PartialEq + std::ops::AddAssign<I>,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterators.is_empty() {
            return None;
        }

        loop {
            // Advance all iterators a first time on init.
            if !self.is_init {
                // Initialize the iterator levels
                for (l, iter) in self
                    .iterator_levels
                    .iter_mut()
                    .zip(self.iterators.iter_mut())
                {
                    if let Some(docid) = iter.next() {
                        *l = docid;
                    } else {
                        return None;
                    }
                }
                self.is_init = true;
            } else {
                // We have initialized the iterator levels.
                // Now we need to advance the iterators that are below the watermark.
                for (l, iter) in self
                    .iterator_levels
                    .iter_mut()
                    .zip(self.iterators.iter_mut())
                {
                    if *l >= self.watermark {
                        continue;
                    }
                    // Ok there is a need to advance
                    // We advance at least to the watermark, as there would be
                    // no point to advance to something lower.
                    if let Some(docid) = //iter.next() {
                        iter.find(|d| *d >= self.watermark)
                    {
                        // if docid < *l {
                        //     panic!(
                        //         "Invariant broken: next_docid={} < doc_id={} for iterator {}",
                        //         docid, *l, i
                        //     );
                        // }
                        *l = docid;
                    } else {
                        // If we cannot advance, we are done.
                        return None;
                    }
                }
            }

            /* for (idx, level) in self.iterator_levels.iter_mut().enumerate() {
            match level {
                Some(doc_id) if *doc_id < self.watermark => {
                    let next = self.iterators[idx].next();
                    // Removing this improves the perf..
                    /*if let Some(next_docid) = next {
                        assert!(
                            next_docid >= *doc_id,
                            "Invariant broken: next_docid={} < doc_id={} for iterator {}",
                            next_docid,
                            doc_id,
                            idx
                        );
                    } */
                    *level = next;
                }
                None => {
                    // We need to advance. We never advanced before
                    *level = self.iterators[idx].next();
                }
                _ => continue, // No need to advance
            } */

            // Note this has been returned earlier.
            // Any None means we are done.
            //if self.iterator_levels.iter().any(|id| id.is_none()) {
            //    return None;
            //}

            // None of the levels are None.

            // Next watermark and test for consistency match
            self.watermark = *self
                .iterator_levels
                .iter()
                // unwrap is safe because of the invariant checked above
                .max()
                .unwrap();

            // If all iterators have the same level, return the level.
            if self.iterator_levels.iter().all(|&id| id == self.watermark) {
                // Push the watermark up by for the next round, so all iterators are advanced
                self.watermark += I::one();
                return Some(self.iterator_levels[0]);
            }

            // next iteration will advance the iterators that are late on the watermark.
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_next_matching() {
        let mut i = vec![1, 2, 3, 4, 5, 6].into_iter();
        let v = i.find(|v| *v >= 5);
        assert_eq!(v, Some(5));
        let v = i.find(|v| *v >= 5);
        assert_eq!(v, Some(6));
        let v = i.find(|v| *v >= 5);
        assert!(v.is_none());
    }

    #[test]
    fn test_conjunction_iterator() {
        let mut ci: ConjunctionIterator<std::ops::Range<usize>, usize> =
            ConjunctionIterator::new(vec![]);
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

    // #[test]
    // #[should_panic(expected = "Invariant broken: next_docid=2 < doc_id=4 for iterator 0")]
    // fn test_broken_conjunction_iterator() {
    //     // Broken invariant:
    //     let ci = ConjunctionIterator::new(vec![
    //         vec![0, 2, 3, 4, 2, 6].into_iter(),
    //         vec![1, 3, 7].into_iter(),
    //         vec![1, 2, 9, 3, 5, 8, 8, 9].into_iter(),
    //     ]);
    //     let _ = ci.collect::<Vec<_>>();
    // }
}
