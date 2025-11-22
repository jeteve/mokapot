use std::fmt;

use crate::models::cnf::Clause;
use crate::models::document::Document;
use crate::models::types::{OurRc, OurStr};

#[cfg(feature = "send")]
pub(crate) type ExpanderF = OurRc<dyn Fn(Clause) -> Clause + Send + Sync>;

#[cfg(not(feature = "send"))]
pub(crate) type ExpanderF = OurRc<dyn Fn(Clause) -> Clause>;

#[derive(Clone)]
// Extends Clauses comming from percolated document with extra termqueries.
pub(crate) struct ClauseExpander(pub(crate) ExpanderF);
impl std::fmt::Debug for ClauseExpander {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ClauseExpander")
            .field(&"_OPAQUE FUNCTION_")
            .finish()
    }
}
impl ClauseExpander {
    pub(crate) fn new(f: ExpanderF) -> Self {
        Self(f)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreHeater {
    pub(crate) id: OurStr,
    pub(crate) expand_clause: ClauseExpander,
    pub(crate) must_filter: bool, // must_filter MUST be true when the clause expander is not exact.
}

impl PreHeater {
    pub(crate) fn new(id: OurStr, ce: ClauseExpander) -> Self {
        Self {
            id,
            expand_clause: ce,
            must_filter: false,
        }
    }
    pub(crate) fn with_must_filter(mut self, new_bool: bool) -> Self {
        self.must_filter = new_bool;
        self
    }
}

// A clause is turned into a MatchItem for the
// purpose of indexing a Query. See `percolator.rs`
#[derive(Clone, Debug)]
pub(crate) struct MatchItem {
    pub(crate) must_filter: bool,
    pub(crate) doc: Document,
    pub(crate) preheaters: Vec<PreHeater>,
    pub(crate) cost: u32,
}

impl MatchItem {
    pub(crate) fn new(doc: Document, cost: u32) -> Self {
        MatchItem {
            doc,
            must_filter: false,
            preheaters: vec![],
            cost,
        }
    }

    pub(crate) fn with_preheater(mut self, ph: PreHeater) -> Self {
        self.preheaters.push(ph);
        self
    }

    pub(crate) fn match_all() -> Self {
        // A match all is expansive
        Self::new(Document::match_all(), 10000)
    }

    pub(crate) fn with_must_filter(mut self) -> Self {
        self.must_filter = true;
        self
    }
}
