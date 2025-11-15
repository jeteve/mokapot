// Parsing CNF queries
use chumsky::prelude::*;

#[derive(Debug, PartialEq)]
enum FieldValue {
    Term(String),
    Prefix(String),
    Integer(i64),
}

fn field_value_parser<'src>() -> impl Parser<'src, &'src str, FieldValue> {
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

    let naked_string = none_of([' ', '\t', '\n', '"', '(', ')', ':', '*'])
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
