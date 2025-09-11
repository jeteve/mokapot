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

mod test_itertools {

    #[test]
    fn test_inplace_reduce() {
        use super::InPlaceReduce;

        let sum_all = |a: &mut i32, i: &i32| *a += i;

        let vs: Vec<i32> = vec![];
        assert_eq!(vs.into_iter().reduce_inplace(sum_all), None);

        let vs = vec![1];
        assert_eq!(vs.into_iter().reduce_inplace(sum_all), Some(1));

        let vs = vec![1, 2, 3];
        assert_eq!(vs.into_iter().reduce_inplace(sum_all), Some(6));
    }

    #[test]
    fn test_theswartz() {
        use super::TheShwartz;
        #[derive(Debug)]
        struct Stuff(f64);
        impl Stuff {
            fn to_f64(&self) -> f64 {
                self.0
            }
        }

        let vs: Vec<Stuff> = vec![Stuff(3.0), Stuff(2.0), Stuff(1.0)];
        let sorted = vs
            .iter()
            .schwartzian(|&v| v.to_f64(), f64::total_cmp)
            .collect::<Vec<_>>();

        assert_eq!(
            format!("{:?}", sorted),
            "[Stuff(1.0), Stuff(2.0), Stuff(3.0)]"
        );
    }
}
