use crate::models::cnf::CNFQuery;
use crate::models::documents::Document;
use crate::models::index::{DocId, Index};

use std::rc::Rc;

/**
* A DocPredicate is used by the percolator
* to pre-process the documents.
* It is something that MUST be true about the document
* for it to match.
*
* The Document Query will be enriched with "OR TermQuery(_predicate_true=name, specificity = super high)"
* only if p(Document) is true,
*
* The Query document (to match) will be enriched with _predicate_true=name
*
* Reserve DocPredicate for cases where an index based approach would not work.
* Like comparison queries (PrefixQueries, < num queries and so on.)
*
* For Individual queries:
*  Predicate is given by the query or NONE.. Example, a PrefixQuery.
*
*  For conjunctions, return all predicates from subqueries.
*
*  For disjunction, return all predicates from subqueries.
*
*/
pub struct DocPredicate {
    pub name: Rc<str>,
    //pub query: &'a dyn Query,
    //f: fn(&dyn Query, Document) -> bool,
}

pub trait Query: std::fmt::Debug {
    fn to_cnf(&self) -> CNFQuery;

    ///
    ///An iterator on all the DocIds matching this query in the index.
    ///
    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a>;

    ///
    ///An iterator on all the Documents matching this query in the index.
    ///
    fn docs_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = &'a Document> + 'a> {
        Box::new(
            self.docids_from_index(index)
                .map(|doc_id| index.get_document(doc_id).expect("Document should exist")),
        )
    }

    /**
     Does this query match this document?
     Eventually this is the ultimate truth for the percolator.
    */
    fn matches(&self, d: &Document) -> bool;

    fn doc_enrichers(&self) -> Vec<DocPredicate> {
        todo!()
    }
}
