use expect_test::expect_file;

use crate::{
    lexer::{LexerError, tokenize},
    tokens::Token,
};

fn print_result(result: &Result<Vec<Token>, LexerError>) -> String {
    match result {
        Ok(tokens) => {
            let mut result = "OK\n".to_string();
            for token in tokens {
                use core::fmt::Write;
                writeln!(&mut result, "{token}").expect("Writing to a string won't fail");
            }
            result
        }
        Err(e) => format!("ERROR\n{e:?}\n"),
    }
}

macro_rules! tests {
    ($($name:ident => $file:literal),+,) => {
        $(
            #[test]
            fn $name() {
                let src = include_str!(concat!("../../../tests/smoke/", $file, ".i"));
                ::expect_test::expect_file![concat!(
                    "../../../tests/smoke/", $file ,".lexed"
                )].assert_eq(&print_result(&tokenize(src)))
            }
        )+
    };
}

tests![
    arithmetic_operations => "arithmetic_operations",
    arrays_and_records => "arrays_and_records",
    comparison_operators => "comparison_operators",
    complex_expressions => "complex_expressions",
    conditionals => "conditionals",
    for_loops => "for_loops",
    function_parameters => "function_parameters",
    function_return => "function_return",
    invalid => "invalid",
    logical_operators => "logical_operators",
    operator_precedence => "operator_precedence",
    records => "records",
    recursive_types => "recursive_types",
    shadow => "shadow",
    type_aliases => "type_aliases",
    type_conversions => "type_conversions",
    variable_declarations => "variable_declarations",
    while_loops => "while_loops",
    parse_minus => "parse_minus",
];
