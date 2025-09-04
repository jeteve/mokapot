use std::iter;

use fake::faker::lorem::en::*;
use itertools::kmerge;
use itertools::Itertools;

trait Doggy {
    fn bark(&self) -> String;
}

struct Kennel<T: Doggy> {
    dog: T,
}

#[derive(Default)]
struct Dog {}
impl Doggy for Dog {
    fn bark(&self) -> String {
        "Woof!".to_string()
    }
}

// Needed to implement Doggy for Box<dyn Doggy>
impl<T: Doggy + ?Sized> Doggy for Box<T> {
    fn bark(&self) -> String {
        (**self).bark()
    }
}

#[test]
fn test_boxed_traits() {
    let d = Dog::default();
    let k = Kennel { dog: d };
    assert_eq!(k.dog.bark(), "Woof!".to_string());

    let boxed_dog: Box<dyn Doggy> = Box::new(Dog::default());
    let k2 = Kennel { dog: boxed_dog };
    assert_eq!(k2.dog.bark(), "Woof!".to_string());
}

#[test]
fn test_kmerge() {
    assert_eq!(
        kmerge(vec![vec![1, 2, 3, 4], vec![4, 5, 6]])
            .dedup()
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    )
}

#[test]
fn test_fake() {
    let words: Vec<String> = Words(3..5).fake();
    println!("words {:?}", words);

    let word: String = Word().fake();
    println!("words {:?}", word);

    use fake::Fake;

    let vector = ["apple", "banana", "cherry", "date", "elderberry"];

    // Use range directly with .fake()
    let max_index = vector.len() - 1;
    let index1: usize = (0..=max_index).fake();
    let mut index2: usize = (0..=max_index).fake();

    while index2 == index1 {
        index2 = (0..=max_index).fake();
    }

    //let random_two = [vector[index1], vector[index2]];
}
