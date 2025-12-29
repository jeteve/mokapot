use std::{fmt::Display, num::NonZeroUsize};

use crate::{
    models::percolator_core::{PercolatorConfig, PercolatorCore, PercolatorError, PercolatorStats},
    prelude::{Document, Qid, Query},
};

#[derive(Default)]
/// A builder should you want to build a percolator
/// with different parameters
pub struct PercBuilder<T> {
    config: PercolatorConfig,
    _marker: std::marker::PhantomData<T>,
}

impl<T> PercBuilder<T>
where
    T: std::cmp::Eq + std::hash::Hash,
{
    pub fn build(self) -> PercolatorUid<T> {
        PercolatorUid {
            perc: PercolatorCore::from_config(self.config),
            qid_uid: bimap::BiMap::<Qid, T>::new(),
        }
    }

    /// Sets the expected number of clauses of indexed queries
    /// to the given value. This help minimizing the number of post-match
    /// checks the percolator has to do.
    ///
    /// You'll have to experiment and benchmark to find
    /// the sweet spot for your specific use case.
    ///
    /// The default is 3.
    ///
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    /// use std::num::NonZeroUsize;
    ///
    /// let p = Percolator::builder().n_clause_matchers(NonZeroUsize::new(5).unwrap()).build();
    ///
    /// ```
    pub fn n_clause_matchers(mut self, n: NonZeroUsize) -> Self {
        self.config.n_clause_matchers = n;
        self
    }

    /// Sets the allowed prefix sizes for prefix queries.
    /// See [`PercolatorConfig::prefix_sizes`] for details.
    ///
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    /// let p = Percolator::builder().prefix_sizes(vec![3, 7, 42]).build();
    /// ```
    ///
    pub fn prefix_sizes(mut self, sizes: Vec<usize>) -> Self {
        self.config.prefix_sizes = sizes;
        self
    }
}

/// A Percolator type, with an API compatible with the previous version.
pub type Percolator = PercolatorUid<Qid>;

/// A percolator that allows identifiying queries
/// by a stable user supplied ID.
///
/// This allow removing queries, compacting the percolator,
/// serialising and deserialising it while keeping the same
/// user supplied identifiers.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "T: serde::Serialize + std::cmp::Eq + std::hash::Hash",
        deserialize = "T: serde::Deserialize<'de> + std::cmp::Eq + std::hash::Hash",
    ))
)]
pub struct PercolatorUid<T> {
    perc: PercolatorCore,
    qid_uid: bimap::BiMap<Qid, T>,
}

impl<T> std::default::Default for PercolatorUid<T>
where
    T: std::cmp::Eq + std::hash::Hash,
{
    fn default() -> Self {
        Self {
            perc: PercolatorCore::default(),
            qid_uid: bimap::BiMap::<Qid, T>::new(),
        }
    }
}

impl<T> Display for PercolatorUid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.perc.fmt(f)
    }
}

/// When the type used is Qid, just use this
/// and keep the same interface as the PercolatorCore
impl PercolatorUid<Qid> {
    pub fn add_query(&mut self, q: Query) -> Qid {
        self.safe_add_query(q).unwrap()
    }

    pub fn safe_add_query(&mut self, q: Query) -> Result<Qid, PercolatorError> {
        let qid = self.perc.safe_add_query(q)?;
        self.qid_uid.insert(qid, qid);
        Ok(qid)
    }

    pub fn remove_qid(&mut self, qid: Qid) -> bool {
        self.qid_uid.remove_by_left(&qid);
        self.perc.remove_qid(qid)
    }
}

impl<T> PercolatorUid<T>
where
    T: std::cmp::Eq + std::hash::Hash + Copy + Default,
{
    /// Returns a percolator builder for configurability
    /// Example:
    /// ```
    /// use mokaccino::prelude::*;
    ///
    /// let mut p = Percolator::builder().build();
    ///
    /// ```
    pub fn builder() -> PercBuilder<T> {
        PercBuilder::default()
    }

    /// Index the given query with the user provided ID.
    /// This is useful if queries already have an identifier
    /// in your database for instance.
    ///
    /// You can supply the same ID to override an existing query.
    pub fn safe_index_query_with_uid(&mut self, q: Query, uid: T) -> Result<T, PercolatorError> {
        let qid = self.perc.safe_add_query(q)?;
        if let bimap::Overwritten::Right(old_qid, _) = self.qid_uid.insert(qid, uid) {
            // Remove old QID, as this was an overwrite.
            self.perc.remove_qid(old_qid);
        }
        Ok(uid)
    }

    /// Removes the given User provided ID from
    /// this percolator. True if it was effectively removed.
    /// false if it was absent (already removed, or simply not present).
    pub fn remove_uid(&mut self, uid: T) -> bool {
        if let Some((qid, _)) = self.qid_uid.remove_by_right(&uid) {
            self.perc.remove_qid(qid)
        } else {
            false
        }
    }

    pub fn get_query(&self, uid: T) -> &Query {
        self.safe_get_query(uid).unwrap()
    }

    pub fn safe_get_query(&self, uid: T) -> Option<&Query> {
        let qid = self.qid_uid.get_by_right(&uid)?;
        self.perc.safe_get_query(*qid)
    }

    /// An iterator of the matching queries user provided IDs given the Document.
    pub fn percolate<'b>(&self, d: &'b Document) -> impl Iterator<Item = T> + use<'b, '_, T> {
        self.perc
            .percolate(d)
            .filter_map(|qid| self.qid_uid.get_by_left(&qid))
            .copied()
    }

    pub fn stats(&self) -> &PercolatorStats {
        self.perc.stats()
    }
}
