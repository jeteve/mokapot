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
    /// Creates a new `MatchItem` from a document and a cost.
    ///
    /// The returned item has no preheaters and `must_filter` initialized to `false`. The `cost` parameter sets the item's cost used for ranking or selection.
    ///
    /// # Examples
    ///
    /// ```
    /// let doc = Document::match_all();
    /// let item = MatchItem::new(doc, 42);
    /// assert_eq!(item.cost, 42);
    /// assert!(!item.must_filter);
    /// assert!(item.preheaters.is_empty());
    /// ```
    pub(crate) fn new(doc: Document, cost: u32) -> Self {
        MatchItem {
            doc,
            must_filter: false,
            preheaters: vec![],
            cost,
        }
    }

    /// Appends a `PreHeater` to this `MatchItem`'s list of preheaters and returns the modified item.
    ///
    /// # Parameters
    ///
    /// - `ph`: The `PreHeater` to append.
    ///
    /// # Returns
    ///
    /// The updated `MatchItem` with `ph` appended to its `preheaters`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let item = MatchItem::new(Document::match_all(), 10000);
    /// let ph = /* construct a PreHeater */ unimplemented!();
    /// let item = item.with_preheater(ph);
    /// ```
    pub(crate) fn with_preheater(mut self, ph: PreHeater) -> Self {
        self.preheaters.push(ph);
        self
    }

    /// Creates a MatchItem that matches any document.
    ///
    /// The returned item is an expansive "match all" entry and is constructed with a high cost.
    ///
    /// # Examples
    ///
    /// ```
    /// let _item = MatchItem::match_all();
    /// ```
    pub(crate) fn match_all() -> Self {
        // A match all is expansive
        Self::new(Document::match_all(), 10000)
    }

    /// Mark the `MatchItem` as requiring filtering and return the modified item.
    ///
    /// This sets the internal `must_filter` flag to `true` on the receiver and yields the updated `MatchItem`.
    ///
    /// # Examples
    ///
    /// ```
    /// let item = MatchItem::new(Document::match_all(), 10000).with_must_filter();
    /// assert!(item.must_filter);
    /// ```
    pub(crate) fn with_must_filter(mut self) -> Self {
        self.must_filter = true;
        self
    }
}