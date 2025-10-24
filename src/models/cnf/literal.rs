use std::{fmt, rc::Rc};

use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::models::{
    cnf::Clause,
    document::Document,
    index::Index,
    percolator::{
        PercolatorConfig,
        tools::{ClauseExpander, PreHeater},
    },
    queries::{
        common::DocMatcher,
        ordered::{I64Query, Ordering},
        prefix::PrefixQuery,
        term::TermQuery,
    },
};

// Returns the clipped len to the smallest number
// According to clip sizes.
fn clipped_len(allowed_size: &[usize], len: usize) -> usize {
    *allowed_size
        .iter()
        .filter(|&&f| f <= len)
        .next_back()
        .unwrap_or(&len)
}

fn safe_prefix(s: &str, len: usize) -> std::borrow::Cow<'_, str> {
    s.get(0..len)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or(std::borrow::Cow::Owned(
            s.chars().take(len).collect::<String>(),
        ))
}

fn prefix_query_preheater(allowed_size: &[usize], pq: &PrefixQuery) -> PreHeater {
    let clipped_len = clipped_len(allowed_size, pq.prefix().len());

    let pfield = pq.field().clone();
    let synth_field: Rc<str> = format!("__PREFIX{}__{}", clipped_len, pq.field()).into();
    let id_field = synth_field.clone();

    let expander = move |mut c: Clause| {
        // Find all term queries with the given field, where the term is actually at least
        // as long as the prefix
        // Then turn them into term queries with the synthetic field name
        let new_term_queries = c
            .term_queries_iter()
            .filter(|&tq| tq.field() == pfield && tq.term().len() >= clipped_len)
            .map(|tq| {
                TermQuery::new(
                    synth_field.clone(),
                    safe_prefix(tq.term().as_ref(), clipped_len),
                )
            })
            .collect_vec();

        c.append_termqueries(new_term_queries);
        c
    };

    PreHeater::new(id_field, ClauseExpander::new(Rc::new(expander)))
        .with_must_filter(clipped_len < pq.prefix().len())
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum LitQuery {
    Term(TermQuery),
    Prefix(PrefixQuery),
    IntQuery(I64Query),
}

impl LitQuery {
    // Simple delegation.
    fn matches(&self, d: &Document) -> bool {
        match self {
            LitQuery::Term(tq) => tq.matches(d),
            LitQuery::Prefix(pq) => pq.matches(d),
            LitQuery::IntQuery(oq) => oq.matches(d),
        }
    }

    pub fn term_query(&self) -> Option<&TermQuery> {
        match self {
            LitQuery::Term(tq) => Some(tq),
            _ => None,
        }
    }

    pub fn prefix_query(&self) -> Option<&PrefixQuery> {
        match self {
            LitQuery::Prefix(pq) => Some(pq),
            _ => None,
        }
    }

    // Just to order Litteral for display.
    fn sort_field(&self) -> Rc<str> {
        match self {
            LitQuery::Term(tq) => tq.field(),
            LitQuery::Prefix(pq) => pq.field(),
            LitQuery::IntQuery(oq) => oq.field(),
        }
    }

    // To sort the term of the query in lexicographic order
    fn sort_term(&self) -> Rc<str> {
        match self {
            LitQuery::Term(tq) => tq.term(),
            LitQuery::Prefix(pq) => pq.prefix(),
            LitQuery::IntQuery(oq) => oq.cmp_point().to_string().into(),
        }
    }
}

impl fmt::Display for LitQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LitQuery::Term(tq) => write!(f, "{}={}", tq.field(), tq.term()),
            LitQuery::Prefix(pq) => write!(f, "{}={}*", pq.field(), pq.prefix()),
            LitQuery::IntQuery(oq) => oq.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
       How this literal would turn into a document (field, value) tuple
       when the CNF is indexed for later percolation.
    */
    pub(crate) fn percolate_doc_field_values(
        &self,
        config: &PercolatorConfig,
    ) -> Vec<(Rc<str>, Rc<str>)> {
        match &self.query {
            LitQuery::Term(tq) => vec![(tq.field(), tq.term())],
            LitQuery::Prefix(pq) => {
                // Logic to index prefix query:
                // clip the prefix to a fixed set of sizes,
                // knowing we will use the same set of sizes for the preheaters
                // and do a last match check on the document.
                let clipped_len = clipped_len(config.prefix_sizes(), pq.prefix().len());

                vec![(
                    format!("__PREFIX{}__{}", clipped_len, pq.field()).into(),
                    pq.prefix()
                        .chars()
                        .take(clipped_len)
                        .collect::<String>()
                        .into(),
                )]
            }
            LitQuery::IntQuery(oq) => {
                // Logic to index numeric query:
                // Always index Lower, equal and greater than,
                // knowing preheaters will generate the lower, equal and greater than
                let tuples: Vec<(Rc<str>, Rc<str>)> = ["LT", "EQ", "GT"]
                    .iter()
                    .map(|c| {
                        (
                            format!("__INT_{}{}__{}", c, oq.cmp_point(), oq.field()).into(),
                            "true".into(),
                        )
                    })
                    .take(3)
                    .collect();
                match oq.cmp_ord() {
                    Ordering::GT => tuples[2..=2].into(),
                    Ordering::LT => tuples[0..=0].into(),
                    Ordering::GE => tuples[1..=2].into(),
                    Ordering::LE => tuples[0..=1].into(),
                    Ordering::EQ => tuples[1..=1].into(),
                }
            }
        }
    }

    pub(crate) fn preheater(&self, config: &PercolatorConfig) -> Option<PreHeater> {
        match &self.query {
            LitQuery::Prefix(pq) => Some(prefix_query_preheater(config.prefix_sizes(), pq)),
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

mod test {
    #[test]
    #[allow(dead_code)]
    fn test_clip() {
        use super::*;
        let sizes = &[1, 2, 3, 5, 8, 89, 1597];
        assert_eq!(clipped_len(sizes, 0), 0);
        assert_eq!(clipped_len(sizes, 1), 1);
        assert_eq!(clipped_len(sizes, 2), 2);

        assert_eq!(clipped_len(sizes, 3), 3);
        assert_eq!(clipped_len(sizes, 4), 3);
        assert_eq!(clipped_len(sizes, 5), 5);
        assert_eq!(clipped_len(sizes, 6), 5);
        assert_eq!(clipped_len(sizes, 7), 5);
        assert_eq!(clipped_len(sizes, 8), 8);
        assert_eq!(clipped_len(sizes, 100), 89);
        assert_eq!(clipped_len(sizes, 2000), 1597);
    }
}
