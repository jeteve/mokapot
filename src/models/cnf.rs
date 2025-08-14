// Strongly inspired by https://www.cs.jhu.edu/~jason/tutorials/convert-to-CNF.html
use crate::models::queries::TermQuery;

use itertools::Itertools;

use std::fmt;

#[derive(Debug, Clone)]
struct Literal(TermQuery);

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.0.field(), self.0.term())
    }
}

#[derive(Debug, Clone)]
struct Clause(Vec<Literal>);
impl Clause {
    fn from_clauses(clauses: Vec<Clause>) -> Self {
        Self(clauses.into_iter().flat_map(|c| c.0).collect())
    }
}
impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(OR {})", self.0.iter().map(|l| l.to_string()).join(" "))
    }
}

#[derive(Debug)]
pub struct CNFQuery(Vec<Clause>);
impl fmt::Display for CNFQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(AND {})",
            self.0.iter().map(|c| c.to_string()).join(" ")
        )
    }
}

impl CNFQuery {
    pub fn from_literal(q: TermQuery) -> Self {
        Self(vec![Clause(vec![Literal(q)])])
    }
    pub fn from_and(cnf_qs: Vec<CNFQuery>) -> Self {
        Self(cnf_qs.into_iter().flat_map(|cnfq| cnfq.0).collect())
    }

    pub fn from_or(cnf_qs: Vec<CNFQuery>) -> Self {
        // Combine all CNF queries into a single CNF query
        Self(
            cnf_qs
                .into_iter()
                .map(|cnfq| cnfq.0.into_iter())
                .multi_cartesian_product()
                .map(|clauses| {
                    // Combine the clauses into one
                    Clause::from_clauses(clauses)
                })
                .collect(),
        )
    }

    pub fn from_or_two(a: CNFQuery, b: CNFQuery) -> Self {
        Self::from_or(vec![a, b])
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_literal() {
        use super::*;
        let term_query = TermQuery::new("field".into(), "value".into());
        let cnf_query = CNFQuery::from_literal(term_query);
        assert_eq!(cnf_query.0.len(), 1);
        assert_eq!(cnf_query.0[0].0.len(), 1);
        assert_eq!(cnf_query.to_string(), "(AND (OR field=value))");
    }

    #[test]
    fn test_from_and() {
        use super::*;
        let term_query1 = TermQuery::new("field1".into(), "value1".into());
        let term_query2 = TermQuery::new("field2".into(), "value2".into());
        let cnf_query1 = CNFQuery::from_literal(term_query1);
        let cnf_query2 = CNFQuery::from_literal(term_query2);
        let combined = CNFQuery::from_and(vec![cnf_query1, cnf_query2]);
        assert_eq!(combined.0.len(), 2);
        assert_eq!(combined.0[0].0.len(), 1);
        // Structure would be:
        assert_eq!(
            combined.to_string(),
            "(AND (OR field1=value1) (OR field2=value2))"
        );
    }

    #[test]
    fn test_from_or() {
        use super::*;
        let x = TermQuery::new("X".into(), "x".into());
        let y = TermQuery::new("Y".into(), "y".into());
        let cnf_query1 = CNFQuery::from_literal(x.clone());
        let cnf_query2 = CNFQuery::from_literal(y.clone());
        let combined = CNFQuery::from_or_two(cnf_query1, cnf_query2);
        assert_eq!(combined.0.len(), 1); // Only one clause in the top level and.
        assert_eq!(combined.0[0].0.len(), 2); // Two litteral in the clause.
                                              // In this shape: AND (OR field1:value1 field2:value2)
        assert_eq!(combined.to_string(), "(AND (OR X=x Y=y))");

        // (x AND Y) OR Z:
        // The Z
        let z = TermQuery::new("Z".into(), "z".into());
        let q = CNFQuery::from_or_two(
            CNFQuery::from_and(vec![
                CNFQuery::from_literal(x.clone()),
                CNFQuery::from_literal(y.clone()),
            ]),
            CNFQuery::from_literal(z.clone()),
        );
        assert_eq!(q.to_string(), "(AND (OR X=x Z=z) (OR Y=y Z=z))");

        // (X OR Y) OR Z
        let q = CNFQuery::from_or_two(
            CNFQuery::from_or_two(
                CNFQuery::from_literal(x.clone()),
                CNFQuery::from_literal(y.clone()),
            ),
            CNFQuery::from_literal(z.clone()),
        );
        // Note how the parentheses are removed magically
        assert_eq!(q.to_string(), "(AND (OR X=x Y=y Z=z))");

        // X AND (Y OR (Z AND W))
        let w = TermQuery::new("W".into(), "w".into());
        let q = CNFQuery::from_and(vec![
            CNFQuery::from_literal(x.clone()),
            CNFQuery::from_or(vec![
                CNFQuery::from_literal(y.clone()),
                CNFQuery::from_and(vec![
                    CNFQuery::from_literal(z.clone()),
                    CNFQuery::from_literal(w.clone()),
                ]),
            ]),
        ]);
        assert_eq!(q.to_string(), "(AND (OR X=x) (OR Y=y Z=z) (OR Y=y W=w))");
    }
}
