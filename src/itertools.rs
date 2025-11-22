use itertools::Itertools;
#[allow(dead_code)]
pub(crate) trait TheShwartz: Iterator + Sized {
    fn schwartzian<F, K, O>(self, fk: F, ord: O) -> impl Iterator<Item = <Self as Iterator>::Item>
    where
        F: Fn(&Self::Item) -> K,
        O: Fn(&K, &K) -> std::cmp::Ordering,
    {
        self.map(|i| (fk(&i), i))
            .sorted_by(|(ka, _ia), (kb, _ib)| ord(ka, kb))
            .map(|(_k, i)| i)
    }
}

pub(crate) trait InPlaceReduce: Iterator + Sized {
    // The closure it takes should return true if it wants to stop
    // reducing.
    fn reduce_inplace<F>(mut self, mut f: F) -> Option<<Self as Iterator>::Item>
    where
        F: FnMut(&mut <Self as Iterator>::Item, &<Self as Iterator>::Item) -> bool,
    {
        match self.next() {
            Some(mut i) => {
                // This iterator has a first item. Consume the rest,
                // stopping the computation
                self.any(|e| f(&mut i, &e));
                Some(i)
            }
            _ => None, // This iterator did not have any items.
        }
    }
}

impl<T> TheShwartz for T where T: Iterator + Sized {}
impl<T> InPlaceReduce for T where T: Iterator + Sized {}

pub(crate) trait Fiboable:
    num_traits::Zero + num_traits::One + num_traits::CheckedAdd + num_traits::CheckedNeg + Copy
{
}

// Blanket Fiboable
impl<T> Fiboable for T where
    T: num_traits::Zero + num_traits::One + num_traits::CheckedAdd + num_traits::CheckedNeg + Copy
{
}

#[allow(dead_code)]
pub(crate) struct Fibo<T: Fiboable> {
    current: T,
    next: T,
}

#[allow(dead_code)]
impl<T: Fiboable> Fibo<T>
where
    T: num_traits::Zero + num_traits::One,
{
    pub(crate) fn new() -> Self {
        Fibo {
            current: T::zero(),
            next: T::one(),
        }
    }
}

impl<T> Iterator for Fibo<T>
where
    T: Fiboable,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let new_next = self.current.checked_add(&self.next)?;
        self.current = self.next;
        self.next = new_next;
        Some(new_next)
    }
}

pub(crate) fn fibo_floor<T: PartialOrd + Fiboable>(n: T) -> T {
    if n < T::zero() {
        fibo_ceil(n.checked_neg().expect("n should be negatable"))
            .checked_neg()
            .unwrap()
    } else {
        let f = Fibo::<T>::new();
        f.filter(|&fi| fi <= n).last().unwrap_or(T::zero())
    }
}

pub(crate) fn fibo_ceil<T: PartialOrd + Fiboable>(n: T) -> T {
    if n < T::zero() {
        fibo_floor(n.checked_neg().expect("n should be negatable"))
            .checked_neg()
            .unwrap()
    } else {
        let mut f = Fibo::<T>::new();
        f.find(|&fi| fi >= n).unwrap()
    }
}

#[cfg(test)]
mod test_itertools {
    use super::*;

    #[test]
    fn test_fibo_bounds() {
        // See https://www.math.net/list-of-fibonacci-numbers
        assert_eq!(fibo_floor(-1), -1);
        assert_eq!(fibo_ceil(-1), -1);

        assert_eq!(fibo_floor(-10), -13);
        assert_eq!(fibo_ceil(-10), -8);

        assert_eq!(fibo_floor(0), 0);
        assert_eq!(fibo_ceil(0), 1);
        assert_eq!(fibo_floor(1), 1);
        assert_eq!(fibo_ceil(1), 1);
        assert_eq!(fibo_floor(2), 2);
        assert_eq!(fibo_ceil(2), 2);
        assert_eq!(fibo_floor(3), 3);
        assert_eq!(fibo_ceil(3), 3);
        assert_eq!(fibo_floor(4), 3);
        assert_eq!(fibo_ceil(4), 5);
        assert_eq!(fibo_floor(5), 5);
        assert_eq!(fibo_ceil(5), 5);
        assert_eq!(fibo_floor(10), 8);
        assert_eq!(fibo_ceil(10), 13);
        assert_eq!(fibo_floor(5000), 4181);
        assert_eq!(fibo_ceil(5000), 6765);

        assert_eq!(fibo_floor(320000), 317811);
        assert_eq!(fibo_ceil(320000), 514229);

        assert_eq!(fibo_floor::<u64>(1836311904), 1836311903);
        assert_eq!(fibo_ceil::<u64>(1836311904), 2971215073);
    }
    #[test]
    fn test_fibo() {
        use super::Fibo;
        let all_usize = Fibo::<i64>::new();
        assert_eq!(all_usize.take(5).collect::<Vec<_>>(), vec![1, 2, 3, 5, 8]);
    }

    #[test]
    fn test_inplace_reduce() {
        use super::InPlaceReduce;

        let sum_all = |a: &mut i32, i: &i32| {
            *a += i;
            false
        };

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
