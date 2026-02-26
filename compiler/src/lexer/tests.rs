use crate::lexer::Lexer;

fn lex(src: &str) -> String {
    let mut result = String::new();
    for token in Lexer::from(src) {
        use core::fmt::Write;
        writeln!(&mut result, "{token}").expect("Writing to a string won't fail");
    }
    result
}

macro_rules! tests {
    ($($name:ident => $file:literal),+,) => {
        $(
            #[test]
            fn $name() {
                let src = include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../tests/src/",
                    $file, ".i"
                ));
                ::expect_test::expect_file![concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../tests/lexer/",
                    $file ,".txt"
                )].assert_eq(&lex(src))
            }
        )+
    };
}

// If the tests are failing because of outdates expected output, run
// ```shell
// UPDATE_EXPECT=1 cargo test
// ```
tests! [
    arithmetic_operations => "arithmetic_operations",
    arrays_and_records => "arrays_and_records",
    comparison_operators => "comparison_operators",
    complex_expressions => "complex_expressions",
    conditionals => "conditionals",
    deep_conditionals => "deep_conditionals",
    for_loops => "for_loops",
    function_parameters => "function_parameters",
    function_return => "function_return",
    identifiers => "identifiers",
    invalid => "invalid",
    lexer_invalid => "lexer_invalid",
    logical_operators => "logical_operators",
    nested_control => "nested_control",
    operator_precedence => "operator_precedence",
    parse_minus => "parse_minus",
    real_literals => "real_literals",
    records => "records",
    recursive_types => "recursive_types",
    shadow => "shadow",
    type_aliases => "type_aliases",
    type_conversions => "type_conversions",
    variable_declarations => "variable_declarations",
    while_loops => "while_loops",
];
