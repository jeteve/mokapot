// Parsing CNF queries
use chumsky::prelude::*;

use crate::{models::cnf, prelude::CNFQueryable};

impl std::str::FromStr for cnf::Query {
    type Err = String; // A newline delimited string, with all parsing errors.

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = query_parser();
        p.parse(s)
            .into_result()
            .map_err(|e| {
                e.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .map(|astq| astq.to_cnf())
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Query {
    Neg(Box<Query>),
    Atom(String, Operator, FieldValue),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
}

fn atom_to_cnf(field: &str, operator: &Operator, field_value: &FieldValue) -> cnf::Query {
    match (&operator, &field_value) {
        // A prefix ALWAYS give a prefix, regardless of operator used.
        // It is a bit dirty, but will fix in the future.
        (Operator::H3Inside, FieldValue::Term(t)) => field.has_value(t.clone()),
        (Operator::H3Inside, FieldValue::Integer(i)) => field.has_value(i.to_string()),
        (_, FieldValue::Prefix(p)) => field.has_prefix(p.clone()),
        (_, FieldValue::Term(t)) => field.has_value(t.clone()),
        // Fallback to term style query in case there is ':123'
        (Operator::Colon, FieldValue::Integer(i)) => field.has_value(i.to_string()),
        (Operator::Lt, FieldValue::Integer(i)) => field.i64_lt(*i),
        (Operator::Le, FieldValue::Integer(i)) => field.i64_le(*i),
        (Operator::Eq, FieldValue::Integer(i)) => field.i64_eq(*i),
        (Operator::Ge, FieldValue::Integer(i)) => field.i64_ge(*i),
        (Operator::Gt, FieldValue::Integer(i)) => field.i64_gt(*i),
    }
}

impl Query {
    fn to_cnf(&self) -> cnf::Query {
        match &self {
            Query::Neg(query) => !query.to_cnf(),
            Query::Atom(field, operator, field_value) => atom_to_cnf(field, operator, field_value),
            Query::And(query, query1) => query.to_cnf() & query1.to_cnf(),
            Query::Or(query, query1) => query.to_cnf() | query1.to_cnf(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Operator {
    Colon,
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    H3Inside,
}

#[derive(Debug, PartialEq, Clone)]
enum FieldValue {
    Term(String),
    Prefix(String),
    Integer(i64),
}

type MyParseError<'src> = extra::Err<Rich<'src, char>>;

fn query_parser<'src>() -> impl Parser<'src, &'src str, Query, MyParseError<'src>> {
    recursive(|expr| {
        let recursive_atom = atom_parser()
            .or(expr.delimited_by(just('('), just(')')))
            .padded();

        let unary = text::ascii::keyword("NOT")
            .padded()
            .repeated()
            .foldr(recursive_atom, |_op, rhs| Query::Neg(Box::new(rhs)))
            .boxed();

        let product = unary.clone().foldl(
            text::ascii::keyword("AND")
                .to(Query::And as fn(_, _) -> _)
                .then(unary)
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        );

        // sum has less precedence than product. So flat queries
        // is a sum of products.
        product.clone().foldl(
            text::ascii::keyword("OR")
                .to(Query::Or as fn(_, _) -> _)
                .then(product)
                .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        )
    })
    .padded()
}

fn atom_parser<'src>() -> impl Parser<'src, &'src str, Query, MyParseError<'src>> {
    identifier_parser()
        .then(operator_parser())
        .then(field_value_parser())
        .map(|((s, o), v)| Query::Atom(s, o, v))
        .padded()
}

fn operator_parser<'src>() -> impl Parser<'src, &'src str, Operator, MyParseError<'src>> {
    choice((
        just("H3IN").to(Operator::H3Inside),
        just(':').to(Operator::Colon),
        just("<=").to(Operator::Le),
        just(">=").to(Operator::Ge),
        just('<').to(Operator::Lt),
        just('>').to(Operator::Gt),
        just('=').to(Operator::Eq),
    ))
    .padded()
}

static NON_IDENTIFIERS: [char; 11] = [' ', '\t', '\n', '"', '(', ')', ':', '*', '<', '>', '='];

fn identifier_parser<'src>() -> impl Parser<'src, &'src str, String, MyParseError<'src>> {
    none_of(NON_IDENTIFIERS)
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .padded()
}

fn field_value_parser<'src>() -> impl Parser<'src, &'src str, FieldValue, MyParseError<'src>> {
    let term_char = just('\\')
        .ignore_then(any()) // After backslash, accept any character
        .or(none_of('"')); // Or any character that's not a quote, as this is meant to use into a phrase parser.

    let phrase = just('"')
        .ignore_then(term_char.repeated().collect::<String>())
        .then_ignore(just('"').labelled("closing double quote"))
        .labelled("Quote enclosed phrase")
        .then(just('*').or_not())
        .map(|(t, wc)| {
            if wc.is_some() {
                FieldValue::Prefix(t)
            } else {
                FieldValue::Term(t)
            }
        });

    let naked_string = none_of(NON_IDENTIFIERS)
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(just('*').or_not())
        .map(|(t, wc)| {
            if wc.is_some() {
                FieldValue::Prefix(t) // With a wild char, this is ALWAYS a word
            } else {
                // Attempt to parse as i64. If fail, fallback to just string.
                t.parse::<i64>()
                    .map(FieldValue::Integer)
                    .unwrap_or(FieldValue::Term(t))
            }
        });

    choice((phrase, naked_string)).padded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_parser() {
        let p = query_parser();

        assert_eq!(
            p.parse("location H3IN blablabla").output(),
            Some(&Query::Atom(
                "location".to_string(),
                Operator::H3Inside,
                FieldValue::Term("blablabla".into())
            ))
        );

        assert_eq!(
            p.parse("name:abc").output(),
            Some(&Query::Atom(
                "name".to_string(),
                Operator::Colon,
                FieldValue::Term("abc".into())
            ))
        );

        assert_eq!(
            p.parse("location H3IN abc")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR location=abc))"
        );

        assert_eq!(
            p.parse("name:abc").output().unwrap().to_cnf().to_string(),
            "(AND (OR name=abc))"
        );

        assert_eq!(
            p.parse("name:abc AND price<=123").output(),
            Some(&Query::And(
                Box::new(Query::Atom(
                    "name".to_string(),
                    Operator::Colon,
                    FieldValue::Term("abc".into())
                )),
                Box::new(Query::Atom(
                    "price".to_string(),
                    Operator::Le,
                    FieldValue::Integer(123)
                ))
            ))
        );

        assert_eq!(
            p.parse("name:abc AND price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR name=abc) (OR price<=123))"
        );

        assert_eq!(
            p.parse("name:abc AND NOT price<=123 OR colour:blue*")
                .output(),
            Some(&Query::Or(
                Box::new(Query::And(
                    Box::new(Query::Atom(
                        "name".to_string(),
                        Operator::Colon,
                        FieldValue::Term("abc".into())
                    )),
                    Box::new(Query::Neg(Box::new(Query::Atom(
                        "price".to_string(),
                        Operator::Le,
                        FieldValue::Integer(123)
                    ))))
                )),
                Box::new(Query::Atom(
                    "colour".to_string(),
                    Operator::Colon,
                    FieldValue::Prefix("blue".into())
                ))
            ))
        );

        assert_eq!(
            p.parse("name:abc AND NOT price<=123 OR colour:blue*")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR colour=blue* name=abc) (OR colour=blue* ~price<=123))"
        );

        // AND has precedence
        assert_eq!(
            p.parse("colour:blue* OR name:abc AND NOT price<=123")
                .output(),
            Some(&Query::Or(
                Box::new(Query::Atom(
                    "colour".to_string(),
                    Operator::Colon,
                    FieldValue::Prefix("blue".into())
                )),
                Box::new(Query::And(
                    Box::new(Query::Atom(
                        "name".to_string(),
                        Operator::Colon,
                        FieldValue::Term("abc".into())
                    )),
                    Box::new(Query::Neg(Box::new(Query::Atom(
                        "price".to_string(),
                        Operator::Le,
                        FieldValue::Integer(123)
                    ))))
                ))
            ))
        );

        assert_eq!(
            p.parse("colour:blue* OR name:abc AND NOT price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR colour=blue* name=abc) (OR colour=blue* ~price<=123))"
        );

        // Beat precedence with parenthesis
        assert_eq!(
            p.parse("(colour:blue* OR name:abc) AND NOT price<=123")
                .output(),
            Some(&Query::And(
                Box::new(Query::Or(
                    Box::new(Query::Atom(
                        "colour".to_string(),
                        Operator::Colon,
                        FieldValue::Prefix("blue".into())
                    )),
                    Box::new(Query::Atom(
                        "name".to_string(),
                        Operator::Colon,
                        FieldValue::Term("abc".into())
                    ))
                )),
                Box::new(Query::Neg(Box::new(Query::Atom(
                    "price".to_string(),
                    Operator::Le,
                    FieldValue::Integer(123)
                ))))
            ))
        );

        assert_eq!(
            p.parse("(colour:blue* OR name:abc) AND NOT price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR colour=blue* name=abc) (OR ~price<=123))"
        );

        // Try a distributed NOT
        assert_eq!(
            p.parse("NOT (colour:blue* OR name:abc) AND price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR ~colour=blue*) (OR ~name=abc) (OR price<=123))"
        );

        assert_eq!(
            p.parse("NOT (colour:blue* AND name:abc) OR price<=123")
                .output()
                .unwrap()
                .to_cnf()
                .to_string(),
            "(AND (OR ~colour=blue* ~name=abc price<=123))"
        );
    }

    #[test]
    fn test_atom_parser() {
        let p = atom_parser();
        assert_eq!(
            p.parse("  name:abc  ").output(),
            Some(&Query::Atom(
                "name".to_string(),
                Operator::Colon,
                FieldValue::Term("abc".to_string())
            ))
        );

        assert_eq!(
            p.parse("  price <= 123  ").output(),
            Some(&Query::Atom(
                "price".to_string(),
                Operator::Le,
                FieldValue::Integer(123)
            ))
        );

        assert_eq!(
            p.parse("  price<=123  ").output(),
            Some(&Query::Atom(
                "price".to_string(),
                Operator::Le,
                FieldValue::Integer(123)
            ))
        );
    }

    #[test]
    fn test_identifier_parser() {
        let p = identifier_parser();
        assert_eq!(p.parse("abcd").output(), Some(&"abcd".to_string()));
        assert_eq!(p.parse("ab.cd").output(), Some(&"ab.cd".to_string()));
        assert_eq!(p.parse("ab_cd").output(), Some(&"ab_cd".to_string()));
        assert_eq!(p.parse("ab-cd-").output(), Some(&"ab-cd-".to_string()));
        assert_eq!(p.parse("ab-cd-<").output(), None);
    }

    #[test]
    fn test_operator_parser() {
        let p = operator_parser();
        assert_eq!(p.parse(":").output(), Some(&Operator::Colon));
        assert_eq!(p.parse("<").output(), Some(&Operator::Lt));
        assert_eq!(p.parse("> ").output(), Some(&Operator::Gt));
        assert_eq!(p.parse(" =").output(), Some(&Operator::Eq));
        assert_eq!(p.parse("<=  ").output(), Some(&Operator::Le));
        assert_eq!(p.parse("  >=").output(), Some(&Operator::Ge));
        assert_eq!(p.parse(" H3IN ").output(), Some(&Operator::H3Inside));
    }

    #[test]
    fn test_field_value_parser() {
        let parser = field_value_parser();

        assert_eq!(parser.parse("").output(), None);
        assert_eq!(
            parser.parse("abc").output(),
            Some(&FieldValue::Term("abc".to_string()))
        );
        assert_eq!(
            parser.parse("abc*").output(),
            Some(&FieldValue::Prefix("abc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin blanc\"").output(),
            Some(&FieldValue::Term("boudin blanc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin \\\" blanc\"").output(),
            Some(&FieldValue::Term("boudin \" blanc".to_string()))
        );

        assert_eq!(
            parser.parse("\"boudin\\* \\\" blanc\"*").output(),
            Some(&FieldValue::Prefix("boudin* \" blanc".to_string()))
        );

        assert!(parser.parse("\"boudin blanc").has_errors());

        assert_eq!(
            parser.parse("\"123\"").output(),
            Some(&FieldValue::Term("123".to_string()))
        );
        assert_eq!(
            parser.parse("123").output(),
            Some(&FieldValue::Integer(123))
        );
        assert_eq!(
            parser.parse("123*").output(),
            Some(&FieldValue::Prefix("123".to_string()))
        );
        assert_eq!(
            parser.parse("-123abc").output(),
            Some(&FieldValue::Term("-123abc".to_string()))
        );
    }
}
