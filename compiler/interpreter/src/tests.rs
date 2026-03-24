use anyhow::{Context, Error, ensure};
use culpa::throws;

#[throws]
fn interpret(src: &str) -> String {
    let tokens: Vec<_> = lexer::Lexer::from(src).collect();
    let (program, parsing_errors) = parser::parse_program(&tokens).context("Failed to parse")?;
    ensure!(
        parsing_errors.is_empty(),
        "Parsing errors: {parsing_errors:?}"
    );
    let (program, _) = ast::convert(&program).context("Typecheck error")?;
    let mut result: Vec<u8> = vec![];
    crate::interpret(&mut result, &program).context("Interpreter error")?;
    String::from_utf8(result).context("Somehow got non-UTF8 output")?
}

macro_rules! tests {
    ($($name:ident => $file:literal),+,) => {
        $(
            #[test]
            #[throws]
            fn $name() {
                let src = include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../tests/src/",
                    $file, ".i"
                ));
                ::expect_test::expect_file![concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../tests/run/",
                    $file ,".stdout"
                )].assert_eq(&interpret(src)?)
            }
        )+
    };
}

// If the tests are failing because of outdates expected output, run
// ```shell
// UPDATE_EXPECT=1 cargo test
// ```
tests! [
    // arithmetic_operations => "arithmetic_operations",
    // arrays_and_records => "arrays_and_records",
    // comparison_operators => "comparison_operators",
    // complex_expressions => "complex_expressions",
    // conditionals => "conditionals",
    // deep_conditionals => "deep_conditionals",
    // for_loops => "for_loops",
    // function_parameters => "function_parameters",
    // function_return => "function_return",
    // identifiers => "identifiers",
    // invalid => "invalid",
    // lexer_invalid => "lexer_invalid",
    // logical_operators => "logical_operators",
    // nested_control => "nested_control",
    // operator_precedence => "operator_precedence",
    // parse_minus => "parse_minus",
    real_literals => "real_literals",
    // records => "records",
    // recursive_function => "recursive_function",
    // recursive_types => "recursive_types",
    // shadow => "shadow",
    // type_aliases => "type_aliases",
    // type_conversions => "type_conversions",
    // variable_declarations => "variable_declarations",
    // while_loops => "while_loops",
];
