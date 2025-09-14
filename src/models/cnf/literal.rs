use std::{fmt, rc::Rc};

use roaring::RoaringBitmap;

use crate::models::{
    document::Document,
    index::Index,
    queries::{PrefixQuery, Query, TermQuery},
};

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
    pub fn new(negated: bool, tq: LitQuery) -> Self {
        Self { negated, tq }
    }

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
