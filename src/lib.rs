pub mod models;

// This public API is tested in
pub fn add_two(i: i32) -> i32 {
    add_one(add_one(i))
}

fn add_one(i: i32) -> i32 {
    i + 1
}

// Private tests go here
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_add_one() {
        assert_eq!(add_one(1), 2);
    }
}
