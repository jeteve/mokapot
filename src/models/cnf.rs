mod literal;
pub mod parsing;

use literal::*;

use crate::geotools::Meters;
use crate::models::queries::latlng_within::LatLngWithinQuery;
use crate::models::{
    document::Document,
    index::{DocId, Index},
    queries::{
        h3_inside::H3InsideQuery,
        ordered::{OrderedQuery, Ordering},
        prefix::PrefixQuery,
        term::TermQuery,
    },
};

//use fixedbitset::FixedBitSet;
use h3o::{CellIndex, LatLng};
use itertools::Itertools;
use roaring::MultiOps;

use std::fmt;

use crate::models::types::OurStr;

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    pub(crate) fn cost(&self) -> u32 {
        self.literals.iter().map(|l| l.cost()).sum()
    }

    fn term_queries_iter(&self) -> impl Iterator<Item = &TermQuery> {
        self.literals
            .iter()
            .map(|l| l.query())
            .filter_map(|lq| lq.term_query())
    }

    fn prefix_queries_iter(&self) -> impl Iterator<Item = &PrefixQuery> {
        self.literals
            .iter()
            .map(|l| l.query())
            .filter_map(|lq| lq.prefix_query())
    }

    pub(crate) fn add_termquery(&mut self, query: TermQuery) {
        self.literals
            .push(Literal::new(false, LitQuery::Term(query)));
    }

    pub(crate) fn append_literals(&mut self, mut ls: Vec<Literal>) {
        self.literals.append(&mut ls);
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
    pub fn negate(self) -> Query {
        let negated_lits = self
            .literals
            .into_iter()
            .map(|l| Query::from_literal(l.negate()))
            .collect();
        Query::from_and(negated_lits)
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

///
/// A CNFQuery is the query model that mokaccino operates on
/// You can build a CNF query using the CNFQuery methods,
/// or use the shorthands provided by the trait implementation
/// as well as the CNFQueryable trait.
///
/// Example with construtor functions:
/// ```
/// use mokaccino::prelude::*;
///
/// let q = Query::from_and(vec![Query::prefix("field", "some"),
///                                 Query::negation(Query::term("field", "someexclusion"))
///                                ]
///                            );
/// ```
///
/// Example with shorthand using traits:
/// ```
/// use mokaccino::prelude::*;
///
/// let q = "field".has_prefix("some") & ! "field".has_value("someexclusion");
/// ```
///
/// Note that even though you can build a query from a tree-like structure,
/// this type is completely flat and non-dynamic, thanks to Conjunctive Normal Form transformations
///
/// See also <https://www.cs.jhu.edu/~jason/tutorials/convert-to-CNF.html>
///
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Query(Vec<Clause>);
impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(AND {})",
            self.0.iter().map(|c| c.to_string()).join(" ")
        )
    }
}

impl std::str::FromStr for Query {
    type Err = String; // A newline delimited string, with all parsing errors.

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use chumsky::Parser;
        let p = parsing::query_parser();
        p.parse(s)
            .into_result()
            .map_err(|e| {
                e.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .map(|astq| astq.to_cnf())
    }
}

impl Query {
    /// Builds a one term query from a T and U.
    /// Example:
    /// ```
    /// use mokaccino::prelude::Query;
    ///
    /// let q = Query::term("field", "exact_value");
    ///
    /// ```
    pub fn term<T, U>(field: T, value: U) -> Self
    where
        T: Into<OurStr>,
        U: Into<OurStr>,
    {
        Self::from_termquery(TermQuery::new(field, value))
    }

    /// Builds a prefix query from a T and U
    /// Example:
    /// ```
    /// use mokaccino::prelude::Query;
    ///
    /// let q = Query::prefix("field", "prefix");
    /// ```
    pub fn prefix<T, U>(field: T, value: U) -> Self
    where
        T: Into<OurStr>,
        U: Into<OurStr>,
    {
        Self::from_prefixquery(PrefixQuery::new(field, value))
    }

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
    pub fn negation(q: Query) -> Self {
        let clause_negations = q.0.into_iter().map(|c| c.negate());
        Self::from_or(clause_negations.collect()).cleanse()
    }

    fn cleanse(self) -> Self {
        Self(self.0.into_iter().map(|c| c.cleanse()).collect())
    }

    /// conjunction of all the given CNFQueries
    pub fn from_and(qs: Vec<Query>) -> Self {
        Self(qs.into_iter().flat_map(|q| q.0).collect())
    }

    /// Disjunction of all the given CNFQueries
    /// Applies distributivity of Conjunctions over disjunctions
    /// <https://proofwiki.org/wiki/Rule_of_Distribution#Conjunction_Distributes_over_Disjunction>g
    pub fn from_or(qs: Vec<Query>) -> Self {
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
    pub(crate) fn clauses(&self) -> &[Clause] {
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
            .map(|c| crate::models::percolator_core::clause_docs_from_idx(c, index));
        MultiOps::intersection(subits).into_iter()
    }

    pub(crate) fn prefix_queries(&self) -> impl Iterator<Item = &PrefixQuery> {
        self.0.iter().flat_map(|c| c.prefix_queries_iter())
    }
}

pub trait CNFQueryable: Into<OurStr> {
    /// A Query where `"field".has_value("the_value")``
    fn has_value<T: Into<OurStr>>(self, v: T) -> Query;

    /// A Query where `"field".has_prefix("/some/prefix")`
    fn has_prefix<T: Into<OurStr>>(self, v: T) -> Query;

    /// A Query where the field represents an H3 cell index
    /// that is contained within the given `cell`.
    /// Use this for geographic queries.
    fn h3in(self, cell: CellIndex) -> Query;

    /// A Query where the field represents a `h3o::coord::latlng`
    /// ( for instance 54.35499723397377,18.662987684795226 )
    /// with must be in a disk defined by `center` and `radius`.
    fn latlng_within(self, center: LatLng, radius: Meters) -> Query;

    /// A query where the field can represents a signed integer
    /// that has a value strictly lower than `v`.
    fn i64_lt(self, v: i64) -> Query;
    /// A query where the field can represents a signed integer
    /// that has a value lower than or equal to `v`.
    fn i64_le(self, v: i64) -> Query;
    /// A query where the field can represents a signed integer
    /// that has a value equal to `v`.
    fn i64_eq(self, v: i64) -> Query;
    /// A query where the field can represents a signed integer
    /// that has a value greater than or equal to `v`.
    fn i64_ge(self, v: i64) -> Query;
    /// A query where the field can represents a signed integer
    /// that has a value strictly greater than `v`.
    fn i64_gt(self, v: i64) -> Query;
}

impl<T> CNFQueryable for T
where
    T: Into<OurStr>,
{
    fn has_value<U: Into<OurStr>>(self, v: U) -> Query {
        let tq = TermQuery::new(self, v);
        Query::from_termquery(tq)
    }

    fn has_prefix<U: Into<OurStr>>(self, v: U) -> Query {
        let pq = PrefixQuery::new(self, v);
        Query::from_prefixquery(pq)
    }

    fn h3in(self, cell: CellIndex) -> Query {
        let q = H3InsideQuery::new(self, cell);
        Query::from_literal(Literal::new(false, LitQuery::H3Inside(q)))
    }

    fn latlng_within(self, center: LatLng, radius: Meters) -> Query {
        let q = LatLngWithinQuery::new(self, center, radius);
        Query::from_literal(Literal::new(false, LitQuery::LatLngWithin(q)))
    }

    fn i64_lt(self, v: i64) -> Query {
        let q = OrderedQuery::<i64>::new(self, v, Ordering::LT);
        Query::from_literal(Literal::new(false, LitQuery::IntQuery(q)))
    }

    fn i64_le(self, v: i64) -> Query {
        let q = OrderedQuery::<i64>::new(self, v, Ordering::LE);
        Query::from_literal(Literal::new(false, LitQuery::IntQuery(q)))
    }

    fn i64_eq(self, v: i64) -> Query {
        let q = OrderedQuery::<i64>::new(self, v, Ordering::EQ);
        Query::from_literal(Literal::new(false, LitQuery::IntQuery(q)))
    }

    fn i64_ge(self, v: i64) -> Query {
        let q = OrderedQuery::<i64>::new(self, v, Ordering::GE);
        Query::from_literal(Literal::new(false, LitQuery::IntQuery(q)))
    }

    fn i64_gt(self, v: i64) -> Query {
        let q = OrderedQuery::<i64>::new(self, v, Ordering::GT);
        Query::from_literal(Literal::new(false, LitQuery::IntQuery(q)))
    }
}

impl std::ops::BitAnd for Query {
    type Output = Query;
    fn bitand(self, rhs: Self) -> Self::Output {
        Query::from_and(vec![self, rhs])
    }
}

impl std::ops::BitOr for Query {
    type Output = Query;
    fn bitor(self, rhs: Self) -> Self::Output {
        Query::from_or(vec![self, rhs])
    }
}

impl std::ops::Not for Query {
    type Output = Query;
    fn not(self) -> Self::Output {
        Query::negation(self)
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
        let q = bla.has_value(foo);
        assert_eq!(q.to_string(), "(AND (OR taste=sweet))");

        let q = "path".has_prefix("/bla");
        assert!(q.prefix_queries().next().is_some());
        assert_eq!(q.to_string(), "(AND (OR path=/bla*))");

        let q = "some_num".i64_eq(1234);
        assert_eq!(q.to_string(), "(AND (OR some_num==1234))");

        let q = "some_num".i64_lt(1234);
        assert_eq!(q.to_string(), "(AND (OR some_num<1234))");

        let q = "some_num".i64_le(1234);
        assert_eq!(q.to_string(), "(AND (OR some_num<=1234))");

        let q = "some_num".i64_ge(1234);
        assert_eq!(q.to_string(), "(AND (OR some_num>=1234))");

        let q = "some_num".i64_gt(1234);
        assert_eq!(q.to_string(), "(AND (OR some_num>1234))");
    }

    #[test]
    fn test_empty() {
        use super::*;
        let cnf = Query::default();
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
        let xsq = Query::from_or((0..5).map(|i| "X".has_value(format!("x_{}", i))).collect());

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
        use crate::models::percolator_core::clause_docs_from_idx;
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
        use crate::models::queries::common::DocMatcher;
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

        let colour: OurStr = "colour".into();

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
