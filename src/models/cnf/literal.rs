use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::models::{
    queries::{h3_inside::H3InsideQuery, latlng_within::LatLngWithinQuery},
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
    LatLngWithin(LatLngWithinQuery),
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
            LitQuery::H3Inside(_) => 900,  // Will have some preheating, but faster than others.
            LitQuery::LatLngWithin(_) => 1000, // Will have some preheating, but will have some post check
        }
    }

    // Simple delegation.
    fn matches(&self, d: &Document) -> bool {
        match self {
            LitQuery::Term(tq) => tq.matches(d),
            LitQuery::Prefix(pq) => pq.matches(d),
            LitQuery::IntQuery(oq) => oq.matches(d),
            LitQuery::H3Inside(h3i) => h3i.matches(d),
            LitQuery::LatLngWithin(llq) => llq.matches(d),
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
            LitQuery::LatLngWithin(llq) => llq.field(),
        }
    }

    // To sort the term of the query in lexicographic order
    fn sort_term(&self) -> OurStr {
        match self {
            LitQuery::Term(tq) => tq.term(),
            LitQuery::Prefix(pq) => pq.prefix(),
            LitQuery::IntQuery(oq) => oq.cmp_point().to_string().into(),
            LitQuery::H3Inside(h3i) => h3i.cell().to_string().into(),
            LitQuery::LatLngWithin(llq) => {
                format!("{},{}", llq.latlng().to_string(), llq.within().to_string()).into()
            }
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
            LitQuery::LatLngWithin(llq) => llq.fmt(f),
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
            LitQuery::LatLngWithin(llq) => vec![],
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
        let _ = lit.percolate_docs_from_idx(&index); // This will panic.
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
#[cfg(test)]
mod tests_literal {
    use super::*;
    use crate::models::document::Document;
    use crate::models::index::Index;
    use crate::models::percolator::PercolatorConfig;
    use crate::models::queries::{
        ordered::I64Query, ordered::Ordering, prefix::PrefixQuery, term::TermQuery,
    };

    #[test]
    fn test_literal_cost() {
        // Term
        let term_q = TermQuery::new("f", "v");
        let lit_term = Literal::new(false, LitQuery::Term(term_q));
        assert_eq!(lit_term.cost(), 10);
        assert_eq!(lit_term.query().cost(), 10);

        // Prefix
        let prefix_q = PrefixQuery::new("f", "p");
        let lit_prefix = Literal::new(false, LitQuery::Prefix(prefix_q));
        assert_eq!(lit_prefix.cost(), 1000);

        // Negated
        let lit_neg = Literal::new(true, LitQuery::Term(TermQuery::new("f", "v")));
        assert_eq!(lit_neg.cost(), 100000);
    }

    #[test]
    fn test_literal_query_getter() {
        let term_q = TermQuery::new("f", "v");
        let lit = Literal::new(false, LitQuery::Term(term_q.clone()));
        if let LitQuery::Term(tq) = lit.query() {
            assert_eq!(tq.field(), term_q.field());
            assert_eq!(tq.term(), term_q.term());
        } else {
            panic!("Expected TermQuery");
        }
    }

    #[test]
    fn test_percolate_doc_field_values() {
        let config = PercolatorConfig::default();

        // Term
        let lit_term = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        let fvs = lit_term.percolate_doc_field_values(&config);
        assert_eq!(fvs, vec![("f".into(), "v".into())]);

        // Prefix (default sizes usually include small numbers)
        let lit_prefix = Literal::new(false, LitQuery::Prefix(PrefixQuery::new("f", "pre")));
        let fvs = lit_prefix.percolate_doc_field_values(&config);
        // Assuming default config has some prefix sizes.
        // percolate_doc_field_values calls clip_prefix_len.
        // It returns keys like __PREFIX{len}__{field} and value {prefix} clipped.
        assert!(!fvs.is_empty());
        assert!(fvs[0].0.starts_with("__PREFIX"));
    }

    #[test]
    fn test_preheater() {
        let config = PercolatorConfig::default();

        // Term - no preheater
        let lit_term = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        assert!(lit_term.preheater(&config).is_none());

        // Prefix - has preheater
        let lit_prefix = Literal::new(false, LitQuery::Prefix(PrefixQuery::new("f", "pre")));
        assert!(lit_prefix.preheater(&config).is_some());

        // Int - has preheater
        let lit_int = Literal::new(
            false,
            LitQuery::IntQuery(I64Query::new("f", 10, Ordering::EQ)),
        );
        assert!(lit_int.preheater(&config).is_some());

        // H3Inside - has preheater (needs h3o dep but H3InsideQuery constructs it)
        // Skipping complex setup for H3Inside preheater verification unless needed for coverage
    }

    #[test]
    fn test_negate_and_is_negated() {
        let lit = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        assert!(!lit.is_negated());

        let neg_lit = lit.negate();
        assert!(neg_lit.is_negated());

        let double_neg = neg_lit.negate();
        assert!(!double_neg.is_negated());
    }

    #[test]
    fn test_matches() {
        let lit = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        let doc_match = Document::default().with_value("f", "v");
        let doc_miss = Document::default().with_value("f", "x");

        // Positive
        assert!(lit.matches(&doc_match));
        assert!(!lit.matches(&doc_miss));

        // Negated
        let lit_neg = lit.negate();
        assert!(!lit_neg.matches(&doc_match)); // !true -> false
        assert!(lit_neg.matches(&doc_miss)); // !false -> true

        // Bitwise XOR logic check: negated ^ matches
        // false ^ true = true
        // false ^ false = false
        // true ^ true = false
        // true ^ false = true
    }

    #[test]
    fn test_percolate_docs_from_idx() {
        let mut index = Index::default();
        let doc = Document::default().with_value("f", "v");
        let doc_id = index.index_document(&doc);

        let lit = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        let bitmap = lit.percolate_docs_from_idx(&index);
        assert!(bitmap.contains(doc_id));
    }

    #[test]
    fn test_ordering() {
        let l1 = Literal::new(false, LitQuery::Term(TermQuery::new("f", "a")));
        let l2 = Literal::new(false, LitQuery::Term(TermQuery::new("f", "b")));
        let l3 = Literal::new(false, LitQuery::Term(TermQuery::new("g", "a")));

        assert!(l1 < l2);
        assert!(l2 > l1);
        assert!(l1 < l3); // Field compare first

        // Ordering does not depend on negated status based on implementation
        // It delegates to query.sort_field() and query.sort_term()
    }

    #[test]
    fn test_partial_ord() {
        let l1 = Literal::new(false, LitQuery::Term(TermQuery::new("f", "a")));
        let l2 = Literal::new(false, LitQuery::Term(TermQuery::new("f", "b")));
        assert_eq!(l1.partial_cmp(&l2), Some(std::cmp::Ordering::Less));
    }

    #[test]
    fn test_display() {
        let lit = Literal::new(false, LitQuery::Term(TermQuery::new("f", "v")));
        assert_eq!(format!("{}", lit), "f=v");

        let lit_neg = lit.negate();
        assert_eq!(format!("{}", lit_neg), "~f=v");
    }

    #[test]
    fn test_litquery_methods() {
        let term_q = TermQuery::new("f", "v");
        let lit_q = LitQuery::Term(term_q.clone());

        assert!(lit_q.term_query().is_some());
        assert!(lit_q.prefix_query().is_none());

        let prefix_q = PrefixQuery::new("f", "p");
        let lit_pq = LitQuery::Prefix(prefix_q);
        assert!(lit_pq.term_query().is_none());
        assert!(lit_pq.prefix_query().is_some());
    }
}

#[cfg(test)]
mod tests_literal_preheater {
    use super::*;
    use crate::models::cnf::Clause;
    use crate::models::queries::{
        ordered::{I64Query, Ordering},
        prefix::PrefixQuery,
    };

    // Testing logic of intcmp_query_preheater
    #[test]
    fn test_intcmp_preheater_logic() {
        // We need to verify that the expander logic correctly checks iv <= cmp_point and iv >= cmp_point
        // The expander is an opaque function, but we can call it on a Clause with TermQueries representing document fields.

        // Case 1: LE (Less or Equal)
        // cmp_point will be fibo_ceil(10) = 13 (depending on fibo impl)
        // Let's check fibo values first if we can, but assuming fibo logic is correct.
        // fibo_ceil(10) -> 13
        // So LE 10 means we index with __INT_LE_13__field
        // And preheater expander should produce that term if the document value <= 13.

        let q = I64Query::new("f", 10, Ordering::LE);
        let ph = intcmp_query_preheater(&q);
        let expander = ph.expand_clause;

        // Document with value 10 (should match)
        let clause = Clause::from_termqueries(vec![TermQuery::new("f", "10")]);
        let expanded = (expander.0)(clause);
        // Should contain __INT_LE_13__f=true
        assert!(expanded.literals().iter().any(|l| {
            l.query()
                .term_query()
                .unwrap()
                .field()
                .starts_with("__INT_LE_")
        }));

        // Document with value 14 (should NOT match) -> 14 > 13
        let clause = Clause::from_termqueries(vec![TermQuery::new("f", "14")]);
        let expanded = (expander.0)(clause);
        // Should NOT contain the synthetic field
        assert!(!expanded.literals().iter().any(|l| {
            l.query()
                .term_query()
                .unwrap()
                .field()
                .starts_with("__INT_LE_")
        }));

        // Case 2: GE (Greater or Equal)
        // fibo_floor(10) -> 8 (assuming)
        // GE 10 means we index with __INT_GE_8__field
        // Preheater expander should produce that term if doc value >= 8.

        let q = I64Query::new("f", 10, Ordering::GE);
        let ph = intcmp_query_preheater(&q);
        let expander = ph.expand_clause;

        // Document with value 10 (should match)
        let clause = Clause::from_termqueries(vec![TermQuery::new("f", "10")]);
        let expanded = (expander.0)(clause);
        assert!(expanded.literals().iter().any(|l| {
            l.query()
                .term_query()
                .unwrap()
                .field()
                .starts_with("__INT_GE_")
        }));

        // Document with value 7 (should NOT match) -> 7 < 8
        let clause = Clause::from_termqueries(vec![TermQuery::new("f", "7")]);
        let expanded = (expander.0)(clause);
        assert!(!expanded.literals().iter().any(|l| {
            l.query()
                .term_query()
                .unwrap()
                .field()
                .starts_with("__INT_GE_")
        }));
    }

    // Testing logic of prefix_query_preheater
    #[test]
    fn test_prefix_preheater_must_filter() {
        let sizes = vec![2, 4];

        // Case 1: Prefix length in sizes (exact match possibility)
        // prefix "abcd" (len 4), clipped len 4. must_filter should be false (optimization)

        let q = PrefixQuery::new("f", "abcd"); // len 4
        let ph = prefix_query_preheater(&sizes, &q);
        assert_eq!(clip_prefix_len(&sizes, 4), 4);
        assert!(!ph.must_filter);

        // Case 2: Prefix length NOT in sizes (must filter)
        // prefix "abcde" (len 5), clipped len 4. must_filter should be true.
        let q = PrefixQuery::new("f", "abcde");
        let ph = prefix_query_preheater(&sizes, &q);
        assert_eq!(clip_prefix_len(&sizes, 5), 4);
        assert!(ph.must_filter);

        // Testing the expander logic too
        // It should match doc values with len >= clipped_len
        let expander = ph.expand_clause;

        // Doc value "abc" (len 3) < 4. Should NOT expand.
        let clause = Clause::from_termqueries(vec![TermQuery::new("f", "abc")]);
        let expanded = (expander.0)(clause);
        // Check no new literals added (or at least no synthetic prefix one)
        assert!(!expanded.literals().iter().any(|l| {
            l.query()
                .term_query()
                .unwrap()
                .field()
                .starts_with("__PREFIX")
        }));

        // Doc value "abcde" (len 5) >= 4. Should expand.
        let clause = Clause::from_termqueries(vec![TermQuery::new("f", "abcde")]);
        let expanded = (expander.0)(clause);
        assert!(expanded.literals().iter().any(|l| {
            l.query()
                .term_query()
                .unwrap()
                .field()
                .starts_with("__PREFIX")
        }));
    }
}
