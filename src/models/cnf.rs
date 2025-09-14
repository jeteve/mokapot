// Strongly inspired by https://www.cs.jhu.edu/~jason/tutorials/convert-to-CNF.html
use crate::models::{
    document::Document,
    index::{DocId, Index},
    queries::{PrefixQuery, Query, TermQuery},
};

//use fixedbitset::FixedBitSet;
use itertools::Itertools;
use roaring::MultiOps;
use roaring::RoaringBitmap;

use std::{fmt, rc::Rc};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LitQuery {
    Term(TermQuery),
    Prefix(PrefixQuery),
}

impl LitQuery {
    // Simple delegation.
    fn matches(&self, d: &Document) -> bool {
        match self {
            LitQuery::Term(tq) => tq.matches(d),
            LitQuery::Prefix(pq) => pq.matches(d),
        }
    }

    fn sort_field(&self) -> Rc<str> {
        match self {
            LitQuery::Term(tq) => tq.field(),
            LitQuery::Prefix(pq) => pq.field(),
        }
    }

    fn sort_term(&self) -> Rc<str> {
        match self {
            LitQuery::Term(tq) => tq.term(),
            LitQuery::Prefix(pq) => pq.prefix(),
        }
    }
}

impl fmt::Display for LitQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LitQuery::Term(tq) => write!(f, "{}={}", tq.field(), tq.term()),
            LitQuery::Prefix(pq) => write!(f, "{}={}*", pq.field(), pq.prefix()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Literal {
    negated: bool,
    tq: LitQuery,
}
impl Literal {
    /*
       How this literal would turn into a document field/value
       when the CNF is indexed for later percolation.
    */
    pub fn percolate_doc_field_value(&self) -> (Rc<str>, Rc<str>) {
        match &self.tq {
            LitQuery::Term(tq) => (tq.field(), tq.term()),
            LitQuery::Prefix(pq) => (
                format!("__PREFIX{}__{}", pq.prefix().len(), pq.field()).into(),
                pq.prefix(),
            ),
        }
    }

    /// The negation of this literal, which is also a literal
    pub fn negate(self) -> Self {
        Self {
            negated: !self.negated,
            tq: self.tq,
        }
    }

    /// Is this negated?
    pub fn is_negated(&self) -> bool {
        self.negated
    }

    pub fn matches(&self, d: &Document) -> bool {
        self.negated ^ self.tq.matches(d)
    }

    // Only used at percolation time
    // The should Never be a prefix query in here.
    pub fn percolate_docs_from_idx<'a>(&self, index: &'a Index) -> &'a RoaringBitmap {
        match &self.tq {
            LitQuery::Term(tq) => tq.docs_from_idx(index),
            _ => panic!("Only term queries are allowed in percolating queries"),
        }
    }
}

impl Ord for Literal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tq
            .sort_field()
            .cmp(&other.tq.sort_field())
            .then_with(|| self.tq.sort_term().cmp(&other.tq.sort_term()))
    }
}

impl PartialOrd for Literal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", if self.is_negated() { "~" } else { "" }, self.tq)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Clause(Vec<Literal>);
impl Clause {
    pub fn from_termqueries(ts: Vec<TermQuery>) -> Self {
        Self(
            ts.into_iter()
                .map(|tq| Literal {
                    negated: false,
                    tq: LitQuery::Term(tq),
                })
                .collect(),
        )
    }

    pub fn add_termquery(&mut self, tq: TermQuery) {
        self.0.push(Literal {
            negated: false,
            tq: LitQuery::Term(tq),
        });
    }

    /// The literals making this clause
    pub fn literals(&self) -> &[Literal] {
        &self.0
    }

    /// A matchall clause
    pub fn match_all() -> Self {
        Self(vec![Literal {
            negated: false,
            tq: LitQuery::Term(TermQuery::match_all()),
        }])
    }

    /// Flattens a collection of clauses. Consumes them
    pub fn from_clauses(cs: Vec<Clause>) -> Self {
        Self(cs.into_iter().flat_map(|c| c.0).collect())
    }

    /*
    pub fn it_from_idx<'a>(&self, index: &'a Index) -> impl Iterator<Item = DocId> + 'a {
        let its = self.0.iter().map(|q| q.tq.docs_from_idx(index).iter());
        itertools::kmerge(its).dedup()
    }*/

    /// Does this clause matches the given document?
    pub fn matches(&self, d: &Document) -> bool {
        self.0.iter().any(|q| q.matches(d))
    }

    /// Applies De Morgan's first law to produce a CNFQuery representing
    /// this negated Clause.
    pub fn negate(self) -> CNFQuery {
        let negated_lits = self
            .0
            .into_iter()
            .map(|l| CNFQuery::from_literal(l.negate()))
            .collect();
        CNFQuery::from_and(negated_lits)
    }

    fn cleanse(self) -> Self {
        Self(self.0.into_iter().unique().collect())
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
    pub fn from_termquery(q: TermQuery) -> Self {
        Self::from_literal(Literal {
            negated: false,
            tq: LitQuery::Term(q),
        })
    }

    pub fn from_prefixquery(q: PrefixQuery) -> Self {
        Self::from_literal(Literal {
            negated: false,
            tq: LitQuery::Prefix(q),
        })
    }

    pub fn from_literal(l: Literal) -> Self {
        Self(vec![Clause(vec![l])])
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
    /// https://proofwiki.org/wiki/Rule_of_Distribution#Conjunction_Distributes_over_Disjunction
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

    /// The docs matching this CNFQuery in the whole index.
    /// This should be rarely used, and is only there for completeness, in the
    /// case you want to use this crate as a very basic search library.
    pub fn docs_from_idx_iter<'a>(
        &self,
        index: &'a Index,
    ) -> impl Iterator<Item = DocId> + use<'a> {
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
        assert_eq!(cnf_query.0[0].0.len(), 1);
        assert_eq!(cnf_query.to_string(), "(AND (OR field=value))");
    }

    #[test]
    fn test_from_and() {
        use super::*;
        let combined = "field1".has_value("value1") & "field2".has_value("value2");
        assert_eq!(combined.0.len(), 2);
        assert_eq!(combined.0[0].0.len(), 1);
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
        assert_eq!(combined.0[0].0.len(), 2); // Two litteral in the clause.
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
