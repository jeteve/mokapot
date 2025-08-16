use super::query::*;
use crate::itertools::TheShwartz;
use crate::models::cnf::CNFQuery;
use crate::models::documents::Document;
use crate::models::index::*;
use crate::models::iterators;

#[derive(Debug)]
pub struct ConjunctionQuery {
    queries: Vec<Box<dyn Query>>,
}
impl ConjunctionQuery {
    pub fn new(queries: Vec<Box<dyn Query>>) -> Self {
        ConjunctionQuery { queries }
    }
}

impl Query for ConjunctionQuery {
    fn matches(&self, d: &Document) -> bool {
        self.queries.iter().all(|q| q.matches(d))
    }

    fn to_document_with_sample(&self, sample: &Index) -> Document {
        if self.queries.is_empty() {
            return Document::default();
        }
        dbg!(self
            .queries
            .iter()
            .schwartzian(
                |q| q.specificity_in_sample(sample),
                |sa, sb| sa.total_cmp(sb).reverse(),
            )
            .next()
            .unwrap())
        .to_document()
    }

    fn to_document(&self) -> Document {
        // This is the to_document of the most specific subquery.
        // ( (f1=a OR f2=b)[0.5] AND f2=c[1] )[1.5]
        // would choose f2=c
        if self.queries.is_empty() {
            return Document::default();
        }

        // Find the most specific subquery and to_document it.
        dbg!(self
            .queries
            .iter()
            .schwartzian(|q| q.specificity(), |sa, sb| sa.total_cmp(sb).reverse())
            .next()
            .unwrap()
            .to_document())
    }

    fn specificity(&self) -> f64 {
        // The sum of sub specifities
        // This is because a conjunction becomes more an more specific
        // as subqueries accumulate.
        self.queries.iter().map(|q| q.specificity()).sum()
    }

    fn docids_from_index<'a>(&self, index: &'a Index) -> Box<dyn Iterator<Item = DocId> + 'a> {
        // Get iterators from all queries
        let iterators: Vec<_> = self
            .queries
            .iter()
            .map(|q| q.docids_from_index(index))
            .collect();

        Box::new(iterators::ConjunctionIterator::new(iterators))
    }

    fn to_cnf(&self) -> CNFQuery {
        CNFQuery::from_and(self.queries.iter().map(|q| q.to_cnf()).collect())
    }
}
