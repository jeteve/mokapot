// Parsing CNF queries
use chumsky::prelude::*;

#[derive(Debug, PartialEq, Clone)]
enum Query {
    Neg(Box<Query>),
    Atom(String, Operator, FieldValue),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
}

#[derive(Debug, PartialEq, Clone)]
enum Operator {
    Colon,
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
}

#[derive(Debug, PartialEq, Clone)]
enum FieldValue {
    Term(String),
    Prefix(String),
    Integer(i64),
}

type MyParseError<'src> = extra::Err<Rich<'src, char>>;

fn query_parser<'src>() -> impl Parser<'src, &'src str, Query, MyParseError<'src>> {
    let field = identifier_parser();
    let operator = operator_parser();
    let value = field_value_parser();

    let atom = field
        .then(operator)
        .then(value)
        .map(|((s, o), v)| Query::Atom(s, o, v))
        .padded();

    let unary = text::ascii::keyword("NOT")
        .padded()
        .repeated()
        .foldr(atom, |_op, rhs| Query::Neg(Box::new(rhs)))
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
    let sum = product.clone().foldl(
        text::ascii::keyword("OR")
            .to(Query::Or as fn(_, _) -> _)
            .then(product)
            .repeated(),
        |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
    );

    sum
}

fn atom_parser<'src>() -> impl Parser<'src, &'src str, Query, MyParseError<'src>> {
    let field = identifier_parser();
    let operator = operator_parser();
    let value = field_value_parser();

    field
        .then(operator)
        .then(value)
        .map(|((s, o), v)| Query::Atom(s, o, v))
        .padded()
}

fn operator_parser<'src>() -> impl Parser<'src, &'src str, Operator, MyParseError<'src>> {
    choice((
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
            p.parse("name:abc").output(),
            Some(&Query::Atom(
                "name".to_string(),
                Operator::Colon,
                FieldValue::Term("abc".into())
            ))
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
