use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::models::{
    queries::h3_inside::H3InsideQuery,
    types::{OurRc, OurStr},
};

use h3o::CellIndex;
use itertools::Itertools;
use roaring::RoaringBitmap;

use crate::{
    itertools::{fibo_ceil, fibo_floor},
    models::{
        cnf::Clause,
        document::Document,
        index::Index,
        percolator::{
            PercolatorConfig,
            tools::{ClauseExpander, PreHeater},
        },
        queries::{
            common::DocMatcher,
            ordered::{I64Query, OrderedQuery, Ordering},
            prefix::PrefixQuery,
            term::TermQuery,
        },
    },
};

// Returns the clipped len to the smallest number
// According to clip sizes.
fn clip_prefix_len(allowed_size: &[usize], len: usize) -> usize {
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

fn h3in_query_preheater(h3i: &H3InsideQuery) -> PreHeater {
    let qfield = h3i.field();
    let qcell = h3i.cell();

    // The expander looks at each of the litteral values of the clause
    // for the field and adds the new Term litterals
    // to match the __H3IN_.. indexed field at the right resolution.
    let expander = move |mut c: Clause| {
        let litfield: OurStr = format!("__H3_IN_{}_{}", qfield, qcell.resolution()).into();
        let new_literals = c
            .term_queries_iter()
            // Filter the right field
            // and parse term to CellIndex
            .filter_map(|tq| {
                (tq.field() == qfield)
                    .then_some(tq.term())
                    .and_then(|v| v.parse::<CellIndex>().ok())
            })
            // Then upgrade the cell to the resolution of the potential parent
            .filter_map(|ci| ci.parent(qcell.resolution()))
            // Then make a new Term query with the right format
            .map(|upgraded_ci| TermQuery::new(litfield.clone(), upgraded_ci.to_string()))
            .map(|q| Literal::new(false, LitQuery::Term(q)))
            .collect_vec();

        c.append_literals(new_literals);
        c
    };

    let id_preheater = format!("H3IN_{}__{}", h3i.field(), qcell.resolution()).into();

    PreHeater::new(id_preheater, ClauseExpander::new(OurRc::new(expander))).with_must_filter(false)
}

// Preheater for interger comparison queries.
fn intcmp_query_preheater(oq: &I64Query) -> PreHeater {
    // ["LT", "EQ", "GT"]
    // synth_field: Rc<str> = format!("__INT_{}_{}__{}", c, oq.cmp_point(), oq.field()).into();
    let oq_field = oq.field();
    let oq_ord = oq.cmp_ord();
    let cmp_point = match oq_ord {
        Ordering::LT | Ordering::LE | Ordering::EQ => fibo_ceil(*oq.cmp_point()),
        Ordering::GT | Ordering::GE => fibo_floor(*oq.cmp_point()),
    };
    let indexed_name: OurStr = match oq_ord {
        Ordering::LT | Ordering::LE | Ordering::EQ => {
            format!("__INT_LE_{}__{}", cmp_point, oq_field)
        }
        Ordering::GT | Ordering::GE => format!("__INT_GE_{}__{}", cmp_point, oq_field),
    }
    .into();

    let expander = move |mut c: Clause| {
        // This clause comes from a document. Find the right field
        let new_literals = c
            .term_queries_iter()
            .filter_map(|tq| {
                (tq.field() == oq_field)
                    .then_some(tq.term())
                    .and_then(|v| v.parse::<i64>().ok())
            })
            // At this point, we have a parseable integer value
            // from the right field.
            .filter_map(|iv|
                // Generate the right kind of match
                match oq_ord {
                    // The query is LE or LT or EQ, we need to use LE
                    // The floor will have been indexed
                    Ordering::LT | Ordering::LE | Ordering::EQ if iv <= cmp_point => {
                        Some(indexed_name.clone())
                    }
                    Ordering::GT | Ordering::GE if iv >= cmp_point => Some(indexed_name.clone()),
                    _ => None,
                })
            .map(|indexed_name| TermQuery::new(indexed_name, "true"))
            .map(|q| Literal::new(false, LitQuery::Term(q)))
            .collect_vec();

        c.append_literals(new_literals);
        c
    };

    // INT_COMPARE is the name of the preheater.
    let id_field = format!("INT_COMPARE_{}__{}", cmp_point, oq.field()).into();
    PreHeater::new(id_field, ClauseExpander::new(OurRc::new(expander))).with_must_filter(true)
}

fn prefix_query_preheater(allowed_size: &[usize], pq: &PrefixQuery) -> PreHeater {
    let clipped_len = clip_prefix_len(allowed_size, pq.prefix().len());

    let pfield = pq.field().clone();
    let synth_field: OurStr = format!("__PREFIX{}__{}", clipped_len, pq.field()).into();
    let id_field = synth_field.clone();

    let expander = move |mut c: Clause| {
        // Find all term queries with the given field, where the term is actually at least
        // as long as the prefix
        // Then turn them into term queries with the synthetic field name
        let new_literals = c
            .term_queries_iter()
            .filter(|&tq| tq.field() == pfield && tq.term().len() >= clipped_len)
            .map(|tq| {
                TermQuery::new(
                    synth_field.clone(),
                    safe_prefix(tq.term().as_ref(), clipped_len),
                )
            })
            .map(|q| Literal::new(false, LitQuery::Term(q)))
            .collect_vec();

        c.append_literals(new_literals);
        c
    };

    PreHeater::new(id_field, ClauseExpander::new(OurRc::new(expander)))
        .with_must_filter(clipped_len < pq.prefix().len())
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum LitQuery {
    Term(TermQuery),
    Prefix(PrefixQuery),
    IntQuery(I64Query),
    H3Inside(H3InsideQuery),
}

impl LitQuery {
    /// Returns the cost of this LitQuery
    /// Term queries are cheap
    /// Prefix and IntQuery are expansive
    fn cost(&self) -> u32 {
        match self {
            LitQuery::Term(_) => 10,
            LitQuery::Prefix(_) => 1000,   // Will have some preheating
            LitQuery::IntQuery(_) => 1000, // Will have some preheating
            LitQuery::H3Inside(_) => 1000, // Will have some preheating, but faster than others.
        }
    }

    // Simple delegation.
    fn matches(&self, d: &Document) -> bool {
        match self {
            LitQuery::Term(tq) => tq.matches(d),
            LitQuery::Prefix(pq) => pq.matches(d),
            LitQuery::IntQuery(oq) => oq.matches(d),
            LitQuery::H3Inside(h3i) => h3i.matches(d),
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
    fn sort_field(&self) -> OurStr {
        match self {
            LitQuery::Term(tq) => tq.field(),
            LitQuery::Prefix(pq) => pq.field(),
            LitQuery::IntQuery(oq) => oq.field(),
            LitQuery::H3Inside(h3i) => h3i.field(),
        }
    }

    // To sort the term of the query in lexicographic order
    fn sort_term(&self) -> OurStr {
        match self {
            LitQuery::Term(tq) => tq.term(),
            LitQuery::Prefix(pq) => pq.prefix(),
            LitQuery::IntQuery(oq) => oq.cmp_point().to_string().into(),
            LitQuery::H3Inside(h3i) => h3i.cell().to_string().into(),
        }
    }
}

impl fmt::Display for LitQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LitQuery::Term(tq) => write!(f, "{}={}", tq.field(), tq.term()),
            LitQuery::Prefix(pq) => write!(f, "{}={}*", pq.field(), pq.prefix()),
            LitQuery::IntQuery(oq) => oq.fmt(f),
            LitQuery::H3Inside(h3i) => h3i.fmt(f),
        }
    }
}

// Turns an H3Inside query into a vector of indexed
// fields.
fn h3i_to_fvs(h3i: &H3InsideQuery) -> Vec<(OurStr, OurStr)> {
    let cell = h3i.cell();
    vec![(
        // We need the field and the resolution,
        // as we will preheat with the resolution.
        format!("__H3_IN_{}_{}", h3i.field(), cell.resolution()).into(),
        // And the value is simply the cell at the resolution.
        cell.to_string().into(),
    )]
}

// Turns an ordered query into a vector of field/values
// for the purpose of indexing the query in the percolator.
fn oq_to_fvs<T: PartialOrd + FromStr + crate::itertools::Fiboable + Display>(
    oq: &OrderedQuery<T>,
) -> Vec<(OurStr, OurStr)> {
    match oq.cmp_ord() {
        Ordering::LT | Ordering::LE | Ordering::EQ => {
            // LT, LE, and EQ, we need to use LE with the fibo ceil,
            // as something that is <= ceil is also potentially <= than the original value.
            let ceil_value = fibo_ceil(*oq.cmp_point());
            vec![(
                format!("__INT_LE_{}__{}", ceil_value, oq.field()).into(),
                "true".into(),
            )]
        }
        Ordering::GT | Ordering::GE => {
            // GE GT, we need to use GE with the fibo floor,
            // as something that is >= floor is potentially also >= than the original value.
            let floor_value = fibo_floor(*oq.cmp_point());
            vec![(
                format!("__INT_GE_{}__{}", floor_value, oq.field()).into(),
                "true".into(),
            )]
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

    pub(crate) fn cost(&self) -> u32 {
        if self.is_negated() {
            100000 // Highest cost.
        } else {
            self.query.cost()
        }
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
    ) -> Vec<(OurStr, OurStr)> {
        match &self.query {
            LitQuery::Term(tq) => vec![(tq.field(), tq.term())],
            LitQuery::Prefix(pq) => {
                // Logic to index prefix query:
                // clip the prefix to a fixed set of sizes,
                // knowing we will use the same set of sizes for the preheaters
                // and do a last match check on the document.
                let clipped_len = clip_prefix_len(config.prefix_sizes(), pq.prefix().len());

                vec![(
                    format!("__PREFIX{}__{}", clipped_len, pq.field()).into(),
                    pq.prefix()
                        .chars()
                        .take(clipped_len)
                        .collect::<String>()
                        .into(),
                )]
            }
            LitQuery::IntQuery(oq) => oq_to_fvs(oq),
            LitQuery::H3Inside(h3i) => h3i_to_fvs(h3i),
        }
    }

    pub(crate) fn preheater(&self, config: &PercolatorConfig) -> Option<PreHeater> {
        match &self.query {
            LitQuery::Prefix(pq) => Some(prefix_query_preheater(config.prefix_sizes(), pq)),
            LitQuery::IntQuery(oq) => Some(intcmp_query_preheater(oq)),
            LitQuery::H3Inside(h3i) => Some(h3in_query_preheater(h3i)),
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::{
        cnf::literal::oq_to_fvs,
        queries::ordered::{OrderedQuery, Ordering},
    };

    #[test]
    #[should_panic]
    fn test_bad_percolate() {
        let lit = Literal::new(false, LitQuery::Prefix(PrefixQuery::new("f", "v")));
        let index = Index::default();
        assert!(lit.percolate_docs_from_idx(&index).is_empty());
    }

    #[test]
    fn test_cost() {
        let lit = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        let neglit = lit.clone().negate();

        assert!(lit.cost() < neglit.cost());
    }
    #[test]
    fn test_oq_to_fvs() {
        for ordering in [
            Ordering::EQ,
            Ordering::LT,
            Ordering::LE,
            Ordering::GE,
            Ordering::GT,
        ] {
            let q = OrderedQuery::new("field", 42, ordering);
            assert!(!oq_to_fvs(&q).is_empty());
        }
    }

    #[test]
    #[allow(dead_code)]
    fn test_clip() {
        use super::*;
        let sizes = &[1, 2, 3, 5, 8, 89, 1597];
        assert_eq!(clip_prefix_len(sizes, 0), 0);
        assert_eq!(clip_prefix_len(sizes, 1), 1);
        assert_eq!(clip_prefix_len(sizes, 2), 2);

        assert_eq!(clip_prefix_len(sizes, 3), 3);
        assert_eq!(clip_prefix_len(sizes, 4), 3);
        assert_eq!(clip_prefix_len(sizes, 5), 5);
        assert_eq!(clip_prefix_len(sizes, 6), 5);
        assert_eq!(clip_prefix_len(sizes, 7), 5);
        assert_eq!(clip_prefix_len(sizes, 8), 8);
        assert_eq!(clip_prefix_len(sizes, 100), 89);
        assert_eq!(clip_prefix_len(sizes, 2000), 1597);
    }
}
