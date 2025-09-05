# Ideas for NOT field=bla

A query NOT field=bla means:

- The value 'bla' is not one of the values of the field (if field present of course).

When a document comes in with `field=bla` -> It needs to match something that excludes the query.

When a document comes in with `field=foo` -> No exclusion.

An index is a Clause index.

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