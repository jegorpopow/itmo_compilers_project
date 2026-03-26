use anyhow::{Context, Error, ensure};
use culpa::throws;

#[throws]
fn interpret(src: &str) -> String {
    let tokens: Vec<_> = lexer::Lexer::from(src).collect();
    let (program, parsing_errors) = parser::parse_program(&tokens).expect("Failed to parse");
    ensure!(
        parsing_errors.is_empty(),
        "Parsing errors: {parsing_errors:?}"
    );
    let (program, _) = ast::convert(&program).expect("Failed to typecheck");
    let mut result: Vec<u8> = vec![];
    crate::interpret(&mut result, &program).context("Interpreter error")?;
    String::from_utf8(result).context("Somehow got non-UTF8 output")?
}

testing::tests! {
    folder = "run"
    extension = "stdout"
    fun = interpret
    pass = [
        // FIXME: null
        // arrays_and_records => "arrays_and_records"

        // FIXME: resolution issues
        // recursive_types => "recursive_types"
        // shadow => "shadow"
        // type_conversions => "type_conversions"
        // variable_declarations => "variable_declarations"

        // FIXME: no main
        // parse_minus => "parse_minus"


        for_loops => "for_loops"
        arithmetic_operations => "arithmetic_operations"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        deep_conditionals => "deep_conditionals"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        operator_precedence => "operator_precedence"
        real_literals => "real_literals"
        records => "records"
        recursive_function => "recursive_function"
        type_aliases => "type_aliases"
        while_loops => "while_loops"
    ]
    fail = [
    ]
}
