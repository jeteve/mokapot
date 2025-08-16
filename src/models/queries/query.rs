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

    /**
     * An iterator on all the DocIds matching this query in the index.
     */
    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a>;

    /**
     * An iterator on all the Documents matching this query in the index.
     */
    fn docs_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = &'a Document> + 'a> {
        Box::new(
            self.docids_from_index(index)
                .map(|doc_id| index.get_document(doc_id).expect("Document should exist")),
        )
    }

    /**
     * A Document for the purpose of indexing in the percolator
     */
    fn to_document(&self) -> Document;

    fn to_document_with_sample(&self, _sample: &Index) -> Document {
        self.to_document()
    }

    /**
     Does this query match this document?
     Eventually this is the ultimate truth for the percolator.
    */
    fn matches(&self, d: &Document) -> bool;

    /**
     The a-priori intricic specificity of a query, regardless of
     any sample index.
    */
    fn specificity(&self) -> f64;

    fn specificity_in_sample(&self, sample: &Index) -> f64 {
        // N Res / Sample size. No specificity (1.0 if no index size)
        //  - ( log(nres) (if not zero) - log(size) )
        // Unknown specificity.
        if sample.is_empty() {
            return 0.0;
        }
        let res_in_sample = self.docids_from_index(sample).count();
        // When there are no result, this is better.
        // Max specificity.
        if res_in_sample == 0 {
            // Max score possible is log2 of sample size
            // Plus one for it to be better than the 1 result only score.
            return (sample.len() as f64).log2() + 1.0;
        }

        // 10 VS 1 is better than 10 VS 5

        // log( 1/10 ) < log( 5 / 10 ) < log( 1 ) == 0

        // The more the results the worse. 2/10 is better than 5/10
        // if there is just one result, this is the max specifity too.
        (sample.len() as f64).log2() - (res_in_sample as f64).log2()
    }

    fn doc_enrichers(&self) -> Vec<DocPredicate> {
        todo!()
    }
}
