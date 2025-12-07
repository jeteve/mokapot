/// Testing oriented utilities.
use crate::prelude::{Query, parsing};

impl Query {
    /// Builds a random query. This is mainly useful for testing and benchmarking.
    /// Example:
    /// ```
    /// use mokaccino::prelude::Query;
    ///
    /// let mut rng = rand::rng();
    /// let q = Query::random(&mut rng);
    /// ```
    pub fn random<U: rand::Rng>(rng: &mut U) -> Self {
        parsing::random_query(rng, 3).to_cnf()
    }

    /// Generate a random query string, compatible with Parsing.
    /// This is mainly useful for testing and benchmarking.
    ///
    /// Example:
    /// ```
    /// use mokaccino::prelude::Query;
    ///
    /// let mut rng = rand::rng();
    /// let s = Query::random_string(&mut rng);
    /// assert!( s.parse::<Query>().is_ok() )
    /// ```
    pub fn random_string<U: rand::Rng>(rng: &mut U) -> String {
        parsing::random_query(rng, 3).to_string()
    }
}
