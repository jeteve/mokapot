use itertools::Itertools;

pub trait TheShwartz: Iterator {
    fn schwartzian<F, K, O>(self, fk: F, ord: O) -> impl Iterator<Item = <Self as Iterator>::Item>
    where
        F: Fn(&Self::Item) -> K,
        O: Fn(&K, &K) -> std::cmp::Ordering,
        Self: Iterator + Sized,
    {
        self.map(|i| (fk(&i), i))
            .sorted_by(|(ka, _ia), (kb, _ib)| ord(ka, kb))
            .map(|(_k, i)| i)
    }
}

pub trait InPlaceReduce: Iterator {
    fn reduce_inplace<F>(mut self, mut f: F) -> Option<<Self as Iterator>::Item>
    where
        F: FnMut(&mut <Self as Iterator>::Item, &<Self as Iterator>::Item),
        Self: Iterator + Sized,
    {
        if let Some(mut i) = self.next() {
            self.for_each(|e| f(&mut i, &e));
            Some(i)
        } else {
            None
        }
    }
}

impl<T> TheShwartz for T where T: Iterator {}
impl<T> InPlaceReduce for T where T: Iterator {}
