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

#[cfg(test)]
mod tests {
    use crate::models::{percolator_core::tools::ClauseExpander, types::OurRc};

    #[test]
    fn clause_expander() {
        let e = ClauseExpander::new(OurRc::new(|c| c));
        let _ = format!("{:?}", e);
    }
}
#[cfg(test)]
mod tests_tools {
    use super::*;
    use crate::models::types::OurRc;

    #[test]
    fn test_clause_expander() {
        let e = ClauseExpander::new(OurRc::new(|c| c));
        let debug = format!("{:?}", e);
        assert!(debug.contains("_OPAQUE FUNCTION_"));
    }

    #[test]
    fn test_preheater_methods() {
        let expander = ClauseExpander::new(OurRc::new(|c| c));
        let ph = PreHeater::new("id".into(), expander.clone());

        assert!(!ph.must_filter);

        let ph2 = ph.with_must_filter(true);
        assert!(ph2.must_filter);

        // Coverage for default false in new
        let ph3 = PreHeater::new("id2".into(), expander.clone());
        assert!(!ph3.must_filter);
    }

    #[test]
    fn test_match_item_methods() {
        let doc = Document::default();
        let mi = MatchItem::new(doc.clone(), 10);

        assert!(!mi.must_filter);
        assert!(mi.preheaters.is_empty());
        assert_eq!(mi.cost, 10);

        let expander = ClauseExpander::new(OurRc::new(|c| c));
        let ph = PreHeater::new("id".into(), expander);

        let mi2 = mi.with_preheater(ph);
        assert_eq!(mi2.preheaters.len(), 1);

        let mi3 = mi2.with_must_filter();
        assert!(mi3.must_filter);
    }

    #[test]
    fn test_match_item_match_all() {
        let mi = MatchItem::match_all();
        assert!(mi.doc.is_match_all());
        assert_eq!(mi.cost, 10000);
        assert!(!mi.must_filter);
    }
}
