use roaring::RoaringBitmap;

use crate::models::{cnf::Clause, document::Document, index::Index};

#[derive(Debug, Default)]
pub(crate) struct ClauseMatcher {
    positive_index: Index,
}

impl ClauseMatcher {
    // Just a proxy to the index.
    pub(crate) fn index_document(&mut self, d: &Document) {
        self.positive_index.index_document(d);
    }

    pub(crate) fn n_indexed(&self) -> usize {
        self.positive_index.len()
    }

    pub(crate) fn clause_docs(&self, c: &Clause) -> RoaringBitmap {
        let mut ret = RoaringBitmap::new();
        c.literals()
            .iter()
            .map(|l| l.percolate_docs_from_idx(&self.positive_index))
            .for_each(|bm| ret |= bm);

        ret
    }
}
