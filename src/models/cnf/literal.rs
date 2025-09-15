use std::{fmt, rc::Rc};

use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::models::{
    cnf::Clause,
    document::Document,
    index::Index,
    percolator::ClauseExpander,
    percolator::PreHeater,
    queries::{common::Query, prefix::PrefixQuery, term::TermQuery},
};

fn prefix_query_preheater(pq: &PrefixQuery) -> PreHeater {
    let plen = pq.prefix().len();
    let pfield = pq.field().clone();
    let synth_field: Rc<str> = format!("__PREFIX{}__{}", plen, pq.field()).into();
    let id_field = synth_field.clone();

    let expander = move |mut c: Clause| {
        // Find all term queries with the given field, where the term is actually at least
        // as long as the prefix
        // Then turn them into term queries with the synthetic field name
        let new_term_queries = c
            .term_queries_iter()
            .filter(|&tq| tq.field() == pfield && tq.term().len() >= plen)
            .map(|tq| {
                TermQuery::new(
                    synth_field.clone(),
                    tq.term().chars().take(plen).collect::<String>(),
                )
            })
            .collect_vec();

        for new_tq in new_term_queries {
            c.add_termquery(new_tq);
        }
        c
    };

    PreHeater::new(id_field, ClauseExpander::new(Rc::new(expander)))
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum LitQuery {
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

    pub fn term_query(&self) -> Option<&TermQuery> {
        match self {
            LitQuery::Term(tq) => Some(tq),
            _ => None,
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
pub(crate) struct Literal {
    negated: bool,
    query: LitQuery,
}
impl Literal {
    pub(crate) fn new(negated: bool, query: LitQuery) -> Self {
        Self { negated, query }
    }

    pub(crate) fn query(&self) -> &LitQuery {
        &self.query
    }

    /*
       When this contains a Prefix query, it needs to return
       a function that will add to all litteral queries that
       match the prefix field a new __PREFIX{}_{} term query
    */

    /*
       How this literal would turn into a document field/value
       when the CNF is indexed for later percolation.
    */
    pub(crate) fn percolate_doc_field_value(&self) -> (Rc<str>, Rc<str>) {
        match &self.query {
            LitQuery::Term(tq) => (tq.field(), tq.term()),
            LitQuery::Prefix(pq) => (
                format!("__PREFIX{}__{}", pq.prefix().len(), pq.field()).into(),
                pq.prefix(),
            ),
        }
    }

    pub(crate) fn preheater(&self) -> Option<PreHeater> {
        match &self.query {
            LitQuery::Prefix(pq) => Some(prefix_query_preheater(pq)),
            _ => None,
        }
    }

    /// The negation of this literal, which is also a literal
    pub(crate) fn negate(self) -> Self {
        Self {
            negated: !self.negated,
            query: self.query,
        }
    }

    /// Is this negated?
    pub(crate) fn is_negated(&self) -> bool {
        self.negated
    }

    pub(crate) fn matches(&self, d: &Document) -> bool {
        self.negated ^ self.query.matches(d)
    }

    // Only used at percolation time
    // The should Never be a prefix query in here.
    pub(crate) fn percolate_docs_from_idx<'a>(&self, index: &'a Index) -> &'a RoaringBitmap {
        match &self.query {
            LitQuery::Term(tq) => tq.docs_from_idx(index),
            _ => panic!("Only term queries are allowed in percolating queries"),
        }
    }
}

impl Ord for Literal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.query
            .sort_field()
            .cmp(&other.query.sort_field())
            .then_with(|| self.query.sort_term().cmp(&other.query.sort_term()))
    }
}

impl PartialOrd for Literal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            if self.is_negated() { "~" } else { "" },
            self.query
        )
    }
}
