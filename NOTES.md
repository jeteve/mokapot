# Design principles

The goal of mokapot is to take queries, index them somehow and have incoming documents match their queries
as fast as possible.

## Entities at play

### What is a query?

A query is represented by a Conjunctive Normal Form boolean expression (see structure CNFQuery). That is an expression of the form:

(a OR b) AND (c OR d)

where a,b,c and d are Literals that can be tested for truthness against a document somehow.

Example of valid litteral:

f1.has_value(v1)  (this can be tested for truthness against a document)
TODO: !f1.has_value(v1) (this can be tested for truthness against a document)
TODO: f1.has_prefix(p1) (this can be tested for truthness against a document)
TODO: f1.is_lower_than(10000) (this can be tested for truthness against a document)

A query can ALWAYS be tested for matching against a single document, as only literal matching need to be addressed. The rest is simple boolean stuff.

A CNFQuery, despite being a static/flat data structure can be built from a tree-like expression for convenience and parsing.

Converting a CNFQuery to a tree like boolean expression is NOT in scope.


### What is a document?

A simple collection of (field,value) tuples, all strings.


### What is an Index?

A data structure that allows to retrieve for a given (field,value) the full list of documents that matches this property.

### What is a Percolator?

A data structure that allows the retrieval of all queries matching a given document. In its trivial form, a percolator is just a query 

## Current Percolator implementation.

Ideas stem from the observation that matching a disjunction of plain litterals is easy:

Let's say the query is (OR f1.has_value(v1))

We can turn this into a document (f1,v1) that we index.

When a document comes in with lets say (f1,v1), we can turn this into a clause (OR f1.has_value(v1)). It is then trivial to run this clause against the index.

So for this simple case, the flows are the following:

Index time: Clause -> Document -> Index
Percolate time: Document -> Clause -> Index -> Matching Clauses -> check match on Document

Observing that an index is ever only good at matching Clause against Documents (or Document against clauses), we can use several indices to deal with CNFs:

N is FIXED for the percolator.

Index time: CNF -> n x Clauses -> N x Documents -> N x Indices. If n < N , a 'MATCH_ALL' docuemnts are backfilled.

Percolate time: Document -> Clause with MATCHALL added -> N x Index -> Conjunction of matches -> Check match of CNF on Document.


# Ideas for NOT field=bla

A query NOT field=bla means:

- The value 'bla' is not one of the values of the field (if field present of course).

When a document comes in with `field=bla` -> It needs to match something that excludes the query.

When a document comes in with `field=foo` -> No exclusion.

An index is a Clause index.

No Vacuous truth.

field=bla IMPLIES the existence of field
~field=bla IMPLIES the existence of field

## the clause (OR ~field=bla)

just means ~field=bla. A document with `field=bla` MUST exclude the clause.

## the clause (OR ~field=bla field=foo)

A document with field=bla MUST exclude the clause, because it
doesn't make the clause true.

A document with ( field=bla , field=foo ) MUST include the clause because it makes the clause true (on field=foo)

A document with ( field=bar ) MUST include the clause, as it makes the clause true (on ~field=bla)

Idea 1: Maintain an exclusion inverted index.

indexing the clause 123: (OR ~field=bla field=foo) means:

Exclusion: (field,bla) -> 123
Inclusion: (field,foo) -> 123
NeedValue: (field) -> 123

Document (field=bla) comes through:

Exclusion = 123
Inclusion = {}
NeedValue = 123

Need value is TRUE , EXCLUDE is TRUE, INCLUDE is FALSE -> FALSE

Document (field=bla , field=foo) comes in:
Exclusion = 123
Inclusion = 123
NeedValue = 123

Inclusions are NOT empty. This is a clause. Inclusion win.

NEEDVALUE is TRUE , EXCLUDE = TRUE , INCLUDE = TRUE -> TRUE

Document (field=bar) comes in..

Exclusion={}
Inclusion={}
NeedValue=123
NEEDVALUE = TRUE, EXCLUDE = FALSE , INCLUDE = FALSE -> TRUE

Document (colour=blue) comes in.
NEEDVALUE = FALSE , EXCLUDE = FALSE , INCLUDE = FALSE -> FALSE

## Clause is (OR ~field=bla colour=blue)
Document (colour=blue) comes in -> Match
Document (field=foo) comes in -> Match
Document (field=bla colour=blue) comes in -> Match (colour=blue is true, field is NOT excluded)
Document (field=bla) comes in -> EXCLUSION.
Document (colour=green field=bar) -> INCLUSION (~field=bla is TRUE because field exists)
Document (colour=green) -> No Match. 