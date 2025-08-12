use fake::faker::lorem::en::*;
use fake::Fake;

#[test]
fn test_fake() {
    let words: Vec<String> = Words(3..5).fake();
    println!("words {:?}", words);

    let word: String = Word().fake();
    println!("words {:?}", word);

    use fake::{Fake, Faker};

    let vector = vec!["apple", "banana", "cherry", "date", "elderberry"];

    // Use range directly with .fake()
    let max_index = vector.len() - 1;
    let index1: usize = (0..=max_index).fake();
    let mut index2: usize = (0..=max_index).fake();

    while index2 == index1 {
        index2 = (0..=max_index).fake();
    }

    let random_two = vec![vector[index1], vector[index2]];
}
