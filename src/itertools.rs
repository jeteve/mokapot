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

impl<T> TheShwartz for T where T: Iterator {}
