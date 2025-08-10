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

pub struct FilterStat<I, P> {
    filter_i: std::iter::Filter<I, P>,
    pre_filter: usize,
    post_filter: usize,
}

impl<I, P> FilterStat<I, P> {
    pub fn new(filter_i: std::iter::Filter<I, P>) -> Self {
        FilterStat {
            filter_i,
            pre_filter: 0,
            post_filter: 0,
        }
    }
    pub fn pre_filter(&self) -> usize {
        self.pre_filter
    }
    pub fn post_filter(&self) -> usize {
        self.post_filter
    }
}

impl<I, P> Iterator for FilterStat<I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.pre_filter += 1;
        let res = self.filter_i.next();
        if res.is_some() {
            self.post_filter += 1;
        }
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_filterstat() {
        let f = (1..10).filter(|n| *n < 5);
        let mut fs = FilterStat::new(f);
        let fnums: Vec<i32> = fs.by_ref().collect();
        assert_eq!(fnums, vec![1, 2, 3, 4]);
        assert!(fs.post_filter() < fs.pre_filter());
    }
}
