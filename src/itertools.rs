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

// Nest an iterator to gather
// various statistics before and after its application.
pub struct IterStat<I>
where
    I: Iterator,
{
    nested_i: I,
    pre_nested: usize,
    post_nested: usize,
}

impl<I> IterStat<I>
where
    I: Iterator,
{
    pub fn new(nested_i: I) -> Self {
        IterStat {
            nested_i,
            pre_nested: 0,
            post_nested: 0,
        }
    }
    pub fn pre_nested(&self) -> usize {
        self.pre_nested
    }
    pub fn post_nested(&self) -> usize {
        self.post_nested
    }
}

impl<I> Iterator for IterStat<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.pre_nested += 1;
        let res = self.nested_i.next();
        if res.is_some() {
            self.post_nested += 1;
        }
        res
    }
}

pub trait Stateable: Iterator + Sized {
    fn with_stat(self) -> IterStat<Self> {
        IterStat::new(self)
    }
}

impl<T> Stateable for T where T: Iterator {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_filterstat() {
        let f = (1..10).filter(|n| *n < 5);
        let mut fs = IterStat::new(f);
        let fnums: Vec<i32> = fs.by_ref().collect();
        assert_eq!(fnums, vec![1, 2, 3, 4]);
        assert!(fs.post_nested() < fs.pre_nested());
    }

    #[test]
    fn test_the_trait() {
        let f = (1..10).filter(|n| *n < 5);
        let mut fs = f.with_stat();
        let fnums: Vec<i32> = fs.by_ref().collect();
        assert_eq!(fnums, vec![1, 2, 3, 4]);
        assert!(fs.post_nested() < fs.pre_nested());
    }
}
