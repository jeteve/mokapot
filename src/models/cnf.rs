// Strongly inspired by https://www.cs.jhu.edu/~jason/tutorials/convert-to-CNF.html
use crate::models::{
    documents::Document,
    index::{DocId, Index},
    iterators::DisjunctionIterator,
    queries::{Query, TermQuery},
};

use itertools::Itertools;

use std::{fmt, iter};

#[derive(Debug, Clone, Eq, PartialEq)]
struct Literal(TermQuery);

impl Ord for Literal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .field()
            .cmp(&other.0.field())
            .then_with(|| self.0.term().cmp(&other.0.term()))
    }
}

impl PartialOrd for Literal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.0.field(), self.0.term())
    }
}

#[derive(Debug, Clone)]
pub struct Clause(Vec<Literal>);
impl Clause {
    pub fn from_termqueries(ts: Vec<TermQuery>) -> Self {
        Self(ts.into_iter().map(Literal).collect())
    }

    pub fn match_all() -> Self {
        Self(vec![Literal(TermQuery::match_all())])
    }

    pub fn from_clauses(cs: Vec<Clause>) -> Self {
        Self(cs.into_iter().flat_map(|c| c.0).collect())
    }

    pub fn dids_from_idx<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + use<'a> {
        DisjunctionIterator::new(self.0.iter().map(|q| q.0.dids_from_idx(index)).collect())
    }

    pub fn matches(&self, d: &Document) -> bool {
        self.0.iter().any(|q| q.0.matches(d))
    }

    pub fn to_document(&self) -> Document {
        self.0.iter().fold(Document::default(), |a, l| {
            a.with_value(l.0.field(), l.0.term())
        })
    }
}
impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(OR {})",
            self.0.iter().sorted().map(|l| l.to_string()).join(" ")
        )
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
    // Just an alias
    pub fn from_termquery(q: TermQuery) -> Self {
        Self::from_literal(q)
    }
    pub fn from_literal(q: TermQuery) -> Self {
        Self(vec![Clause(vec![Literal(q)])])
    }
    pub fn from_and(qs: Vec<CNFQuery>) -> Self {
        Self(qs.into_iter().flat_map(|q| q.0).collect())
    }

    pub fn from_or(qs: Vec<CNFQuery>) -> Self {
        // Combine all CNF queries into a single CNF query
        Self(
            qs.into_iter()
                .map(|q| q.0.into_iter())
                .multi_cartesian_product()
                .map(|cs| {
                    // Combine the clauses into one
                    Clause::from_clauses(cs)
                })
                .collect(),
        )
    }

    pub fn from_or_two(a: CNFQuery, b: CNFQuery) -> Self {
        Self::from_or(vec![a, b])
    }

    /**
      Return an infinite iterator of documents.
      If you pull more document than there are clauses in this query,
      you get match all documents.
      This is bounded to 1000 extra match all documents to avoid infinite loops.
    */
    pub fn to_documents(&self) -> impl Iterator<Item = Document> + use<'_> {
        self.0
            .iter()
            .map(|c| c.to_document())
            .chain(iter::repeat(Document::match_all()).take(1000))
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = CNFQuery(vec![]);
        assert_eq!(cnf.to_string(), "(AND )");
        assert_eq!(cnf.to_documents().next(), Some(Document::match_all()));
    }

    #[test]
    fn test_literal() {
        use super::*;
        let term_query = TermQuery::new("field".into(), "value".into());
        let cnf_query = CNFQuery::from_literal(term_query);
        assert_eq!(cnf_query.0.len(), 1);
        assert_eq!(cnf_query.0[0].0.len(), 1);
        assert_eq!(cnf_query.to_string(), "(AND (OR field=value))");
        let mut docs = cnf_query.to_documents();
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field", "value"))
        );
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
        let mut docs = combined.to_documents();
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field1", "value1"))
        );
        assert_eq!(
            docs.next(),
            Some(Document::default().with_value("field2", "value2"))
        );
        assert_eq!(docs.next(), Some(Document::match_all()));
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
        let mut docs = combined.to_documents();
        assert_eq!(
            docs.next(),
            Some(
                Document::default()
                    .with_value("X", "x")
                    .with_value("Y", "y")
            )
        );
        assert_eq!(docs.next(), Some(Document::match_all()));

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
        let mut docs = q.to_documents();
        assert_eq!(
            docs.next(),
            Some(
                Document::default()
                    .with_value("X", "x")
                    .with_value("Z", "z")
            )
        );
        assert_eq!(
            docs.next(),
            Some(
                Document::default()
                    .with_value("Y", "y")
                    .with_value("Z", "z")
            )
        );
        assert_eq!(docs.next(), Some(Document::match_all()));

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
        assert_eq!(q.to_string(), "(AND (OR X=x) (OR Y=y Z=z) (OR W=w Y=y))");
    }

    // Different values OR
    #[test]
    fn test_or_with_multiple_values() {
        use super::*;
        let xsq = CNFQuery::from_or(
            (0..5)
                .map(|i| TermQuery::new("X".into(), format!("x_{}", i).into()))
                .map(CNFQuery::from_literal)
                .collect(),
        );
        let oney = CNFQuery::from_literal(TermQuery::new("Y".into(), "y".into()));

        let combined = CNFQuery::from_and(vec![xsq, oney]);
        assert_eq!(
            combined.to_string(),
            "(AND (OR X=x_0 X=x_1 X=x_2 X=x_3 X=x_4) (OR Y=y))"
        );
    }
}
