// Strongly inspired by https://www.cs.jhu.edu/~jason/tutorials/convert-to-CNF.html
use crate::models::{
    document::Document,
    index::{DocId, Index},
    queries::{prefix::PrefixQuery, term::TermQuery},
};

//use fixedbitset::FixedBitSet;
use itertools::Itertools;
use roaring::MultiOps;

use std::{fmt, rc::Rc};

mod literal;
use literal::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Clause {
    literals: Vec<Literal>,
}

impl Clause {
    pub(crate) fn from_termqueries(ts: Vec<TermQuery>) -> Self {
        Self {
            literals: ts
                .into_iter()
                .map(|tq| Literal::new(false, LitQuery::Term(tq)))
                .collect(),
        }
    }

    fn term_queries_iter(&self) -> impl Iterator<Item = &TermQuery> {
        self.literals
            .iter()
            .map(|l| l.query())
            .filter_map(|lq| lq.term_query())
    }

    pub(crate) fn add_termquery(&mut self, query: TermQuery) {
        self.literals
            .push(Literal::new(false, LitQuery::Term(query)));
    }

    /// The literals making this clause
    pub(crate) fn literals(&self) -> &[Literal] {
        &self.literals
    }

    /// A matchall clause
    pub fn match_all() -> Self {
        Self {
            literals: vec![Literal::new(false, LitQuery::Term(TermQuery::match_all()))],
        }
    }

    /// Flattens a collection of clauses. Consumes them
    pub fn from_clauses(cs: Vec<Clause>) -> Self {
        Self {
            literals: cs.into_iter().flat_map(|c| c.literals).collect(),
        }
    }

    /// Does this clause matches the given document?
    pub fn matches(&self, d: &Document) -> bool {
        self.literals.iter().any(|q| q.matches(d))
    }

    /// Applies De Morgan's first law to produce a CNFQuery representing
    /// this negated Clause.
    pub fn negate(self) -> CNFQuery {
        let negated_lits = self
            .literals
            .into_iter()
            .map(|l| CNFQuery::from_literal(l.negate()))
            .collect();
        CNFQuery::from_and(negated_lits)
    }

    fn cleanse(self) -> Self {
        Self {
            literals: self.literals.into_iter().unique().collect(),
        }
    }
}

impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(OR {})",
            self.literals
                .iter()
                .sorted()
                .map(|l| l.to_string())
                .join(" ")
        )
    }
}

#[derive(Debug, Default, Clone)]
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
    /// A new CNFQuery from a plain TermQuery
    pub(crate) fn from_termquery(q: TermQuery) -> Self {
        Self::from_literal(Literal::new(false, LitQuery::Term(q)))
    }

    pub(crate) fn from_prefixquery(q: PrefixQuery) -> Self {
        Self::from_literal(Literal::new(false, LitQuery::Prefix(q)))
    }

    pub(crate) fn from_literal(l: Literal) -> Self {
        Self(vec![Clause { literals: vec![l] }])
    }

    /// Applies the second De Morgan law
    /// to build a CNFQuery representing the negation of this one.
    pub fn negation(q: CNFQuery) -> Self {
        let clause_negations = q.0.into_iter().map(|c| c.negate());
        Self::from_or(clause_negations.collect()).cleanse()
    }

    fn cleanse(self) -> Self {
        Self(self.0.into_iter().map(|c| c.cleanse()).collect())
    }

    /// conjunction of all the given CNFQueries
    pub fn from_and(qs: Vec<CNFQuery>) -> Self {
        Self(qs.into_iter().flat_map(|q| q.0).collect())
    }

    /// Disjunction of all the given CNFQueries
    /// Applies distributivity of Conjunctions over disjunctions
    /// <https://proofwiki.org/wiki/Rule_of_Distribution#Conjunction_Distributes_over_Disjunction>
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

    ///
    /// Does this query match a document?
    pub fn matches(&self, d: &Document) -> bool {
        self.0.iter().all(|c| c.matches(d))
    }

    /// The clauses of this CNFQuery
    pub fn clauses(&self) -> &[Clause] {
        &self.0
    }

    // The docs matching this CNFQuery in the whole index.
    // This should be rarely used, and is only there for completeness
    #[allow(dead_code)]
    fn docs_from_idx_iter<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + use<'a> {
        // And multi and between all clauses.
        let subits = self
            .0
            .iter()
            .map(|c| crate::models::percolator::clause_docs_from_idx(c, index));
        MultiOps::intersection(subits).into_iter()
    }
}

pub trait CNFQueryable: Into<Rc<str>> {
    fn has_value<T: Into<Rc<str>>>(self, v: T) -> CNFQuery;
    fn has_prefix<T: Into<Rc<str>>>(self, v: T) -> CNFQuery;
}

impl<T> CNFQueryable for T
where
    T: Into<Rc<str>>,
{
    fn has_value<U: Into<Rc<str>>>(self, v: U) -> CNFQuery {
        let tq = TermQuery::new(self, v);
        CNFQuery::from_termquery(tq)
    }

    fn has_prefix<U: Into<Rc<str>>>(self, v: U) -> CNFQuery {
        let pq = PrefixQuery::new(self, v);
        CNFQuery::from_prefixquery(pq)
    }
}

impl std::ops::BitAnd for CNFQuery {
    type Output = CNFQuery;
    fn bitand(self, rhs: Self) -> Self::Output {
        CNFQuery::from_and(vec![self, rhs])
    }
}

impl std::ops::BitOr for CNFQuery {
    type Output = CNFQuery;
    fn bitor(self, rhs: Self) -> Self::Output {
        CNFQuery::from_or(vec![self, rhs])
    }
}

impl std::ops::Not for CNFQuery {
    type Output = CNFQuery;
    fn not(self) -> Self::Output {
        CNFQuery::negation(self)
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_queryable() {
        use super::CNFQueryable;
        let q = "bla".has_value("foo");

        assert_eq!(q.to_string(), "(AND (OR bla=foo))");
        assert_eq!((!q).to_string(), "(AND (OR ~bla=foo))");

        let bla = "taste".to_string();
        let foo = "sweet".to_string();
        let q2 = bla.has_value(foo);
        assert_eq!(q2.to_string(), "(AND (OR taste=sweet))");
    }

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = CNFQuery::default();
        assert_eq!(cnf.to_string(), "(AND )");
        assert_eq!((!cnf).to_string(), "(AND )");
    }

    #[test]
    fn test_literal() {
        use super::*;
        let cnf_query = "field".has_value("value");
        assert_eq!(cnf_query.0.len(), 1);
        assert_eq!(cnf_query.0[0].literals.len(), 1);
        assert_eq!(cnf_query.to_string(), "(AND (OR field=value))");
    }

    #[test]
    fn test_from_and() {
        use super::*;
        let combined = "field1".has_value("value1") & "field2".has_value("value2");
        assert_eq!(combined.0.len(), 2);
        assert_eq!(combined.0[0].literals.len(), 1);
        // Structure would be:
        assert_eq!(
            combined.to_string(),
            "(AND (OR field1=value1) (OR field2=value2))"
        );

        // De Morgan law
        // NOT (AND C1 C2) = (OR (NOT C1) (NOT C2))
        assert_eq!(
            (!combined).to_string(),
            "(AND (OR ~field1=value1 ~field2=value2))"
        );
    }

    #[test]
    fn test_from_or() {
        use super::*;
        let combined = "X".has_value("x") | "Y".has_value("y");
        assert_eq!(combined.0.len(), 1); // Only one clause in the top level and.
        assert_eq!(combined.0[0].literals.len(), 2); // Two litteral in the clause.
        // In this shape: AND (OR field1:value1 field2:value2)
        assert_eq!(combined.to_string(), "(AND (OR X=x Y=y))");
        // Second De Morgan law
        // NOT A OR B = NOT A AND NOT B
        assert_eq!((!combined).to_string(), "(AND (OR ~X=x) (OR ~Y=y))");

        // (x AND Y) OR (NOT Z):
        let q = ("X".has_value("x") & "Y".has_value("y")) | (!"Z".has_value("z"));

        assert_eq!(q.to_string(), "(AND (OR X=x ~Z=z) (OR Y=y ~Z=z))");
        assert_eq!(
            (!q.clone()).to_string(),
            "(AND (OR ~X=x ~Y=y) (OR ~X=x Z=z) (OR ~Y=y Z=z) (OR Z=z))"
        );

        // (X OR Y) OR Z
        let q = ("X".has_value("x") | "Y".has_value("y")) | "Z".has_value("z");
        // Note how the parentheses are removed magically
        assert_eq!(q.to_string(), "(AND (OR X=x Y=y Z=z))");

        // X AND (Y OR (Z AND W))
        let q =
            "X".has_value("x") & ("Y".has_value("y") | ("Z".has_value("z") & "W".has_value("w")));
        assert_eq!(q.to_string(), "(AND (OR X=x) (OR Y=y Z=z) (OR W=w Y=y))");

        // ( X AND Y ) OR ( Z AND W )
        // Turns into
        let q =
            ("X".has_value("x") & "Y".has_value("y")) | ("Z".has_value("z") & "W".has_value("w"));
        assert_eq!(
            q.to_string(),
            "(AND (OR X=x Z=z) (OR W=w X=x) (OR Y=y Z=z) (OR W=w Y=y))"
        )
    }

    // Different values OR
    #[test]
    fn test_or_with_multiple_values() {
        use super::*;
        let xsq = CNFQuery::from_or((0..5).map(|i| "X".has_value(format!("x_{}", i))).collect());

        let combined = xsq & "Y".has_value("y");
        assert_eq!(
            combined.to_string(),
            "(AND (OR X=x_0 X=x_1 X=x_2 X=x_3 X=x_4) (OR Y=y))"
        );
    }
}

mod test_clause {
    #[test]
    fn test_clause_match() {
        use super::*;
        let d: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("colour", "green")
            .with_value("taste", "sweet");

        let green_or_bitter = Clause::from_termqueries(vec![
            TermQuery::new("colour", "green"),
            TermQuery::new("taste", "bitter"),
        ]);
        assert!(green_or_bitter.matches(&d));

        let red_or_bitter = Clause::from_termqueries(vec![
            TermQuery::new("colour", "red"),
            TermQuery::new("taste", "bitter"),
        ]);
        assert!(!red_or_bitter.matches(&d));
    }

    #[test]
    fn test_clause() {
        use super::*;
        use crate::models::percolator::clause_docs_from_idx;
        use std::collections::HashSet;

        let d: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "sweet");

        let d1: Document = Document::default()
            .with_value("colour", "yellow")
            .with_value("taste", "sour");

        let d2: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "bitter");

        let d3: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "sweet");

        let d4: Document = Document::default()
            .with_value("colour", "yellow")
            .with_value("taste", "bitter");

        let one_clause = Clause::from_termqueries(vec![TermQuery::new("colour", "blue")]);
        assert!(one_clause.matches(&d));

        let mut index = Index::default();
        // Query against the empty index.

        let doc_ids: Vec<_> = clause_docs_from_idx(&one_clause, &index).iter().collect();
        assert!(doc_ids.is_empty());

        let q = TermQuery::new("colour", "blue");
        let q2 = TermQuery::new("taste", "sweet");
        let disq = Clause::from_termqueries(vec![q, q2]);

        assert!(disq.matches(&d));

        let doc_ids: Vec<_> = clause_docs_from_idx(&disq, &index).iter().collect();
        assert!(doc_ids.is_empty());

        index.index_document(&d);
        index.index_document(&d1);
        index.index_document(&d2);
        index.index_document(&d3);
        index.index_document(&d4);

        // colour = blue or taste = sweet.
        let doc_ids: HashSet<DocId> = clause_docs_from_idx(&disq, &index).iter().collect();
        // Notice the order does not matter..
        assert_eq!(doc_ids, HashSet::from([0, 2, 3]));

        // Test the one term disjunction, to check the
        // optmimisation
        let doc_ids: HashSet<DocId> = clause_docs_from_idx(&one_clause, &index).iter().collect();
        assert_eq!(doc_ids, HashSet::from([0, 2, 3]));
    }
}

mod test_queries {

    #[test]
    fn test_term_query() {
        use super::*;
        use crate::models::queries::common::Query;
        let d: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("colour", "green")
            .with_value("taste", "sweet");

        let d2: Document = Document::default()
            .with_value("colour", "yellow")
            .with_value("colour", "green")
            .with_value("taste", "bitter");

        let mut index = Index::default();
        // A query on an empty index.
        let q = "colour".has_value("blue");
        assert_eq!(q.docs_from_idx_iter(&index).count(), 0);

        index.index_document(&d);
        index.index_document(&d2);

        assert!(q.matches(&d));
        assert!(q.docs_from_idx_iter(&index).next().is_some());
        assert_eq!(q.docs_from_idx_iter(&index).count(), 1);

        let colour: Rc<str> = "colour".into();

        let q2 = TermQuery::new(colour, "green");
        assert!(q2.matches(&d));
        assert!(q2.matches(&d2));
        assert_eq!(q2.docs_from_idx(&index).len(), 2);

        let q2 = TermQuery::new("colour", "red");
        assert!(!q2.matches(&d));
        assert!(q2.docs_from_idx(&index).is_empty());
        assert_eq!(q2.docs_from_idx(&index).len(), 0);

        let q3 = TermQuery::new("another_key", "sausage");
        assert!(!q3.matches(&d));
        assert!(q3.docs_from_idx(&index).is_empty());
    }

    #[test]
    fn test_conjunction_query() {
        use super::*;
        let d: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "sweet");

        let d1: Document = Document::default()
            .with_value("colour", "yellow")
            .with_value("taste", "sweet");

        let d2: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "bitter");

        let d3: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "sweet");

        let q = "colour".has_value("blue");
        let q2 = "taste".has_value("sweet");
        let conjunction_query = q & q2;

        assert!(conjunction_query.matches(&d));

        // Index the document
        let mut index = Index::default();
        let doc_ids: Vec<DocId> = conjunction_query.docs_from_idx_iter(&index).collect();
        assert_eq!(doc_ids, vec![] as Vec<DocId>);

        index.index_document(&d);
        index.index_document(&d1);
        index.index_document(&d2);
        index.index_document(&d3);

        let mut doc_ids = conjunction_query.docs_from_idx_iter(&index);
        assert_eq!(doc_ids.next(), Some(0));
        assert_eq!(doc_ids.next(), Some(3));
        assert_eq!(doc_ids.next(), None);
        assert_eq!(doc_ids.next(), None);
    }

    #[test]
    fn test_disjunction_query() {
        use super::*;
        let d: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "sweet");

        let d1: Document = Document::default()
            .with_value("colour", "yellow")
            .with_value("taste", "sour");

        let d2: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "bitter");

        let d3: Document = Document::default()
            .with_value("colour", "blue")
            .with_value("taste", "sweet");

        let d4: Document = Document::default()
            .with_value("colour", "yellow")
            .with_value("taste", "bitter");

        let q = "colour".has_value("blue");
        let q2 = "taste".has_value("sweet");
        let disq = q | q2;
        assert!(disq.matches(&d));

        let mut index = Index::default();
        // Query against the empty index.
        let doc_ids: Vec<_> = disq.docs_from_idx_iter(&index).collect();
        assert!(doc_ids.is_empty());

        index.index_document(&d);
        index.index_document(&d1);
        index.index_document(&d2);
        index.index_document(&d3);
        index.index_document(&d4);

        // colour = blue or taste = sweet.
        let mut doc_ids = disq.docs_from_idx_iter(&index);
        assert_eq!(doc_ids.next(), Some(0));
        assert_eq!(doc_ids.next(), Some(2));
        assert_eq!(doc_ids.next(), Some(3));
        // No more matches!
        assert_eq!(doc_ids.next(), None);
        assert_eq!(doc_ids.next(), None);
    }
}
