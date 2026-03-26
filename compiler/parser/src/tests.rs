use lexer::Lexer;

use crate::{PureParsingResult, parse_program};

fn parse(src: &str) -> PureParsingResult<String> {
    let tokens: Vec<_> = Lexer::from(src).collect();
    parse_program(&tokens).map(|res| format!("{res:#?}\n"))
}

testing::tests! {
    folder = "parser"
    extension = "txt"
    fun = parse
    pass = [
        arithmetic_operations => "arithmetic_operations"
        arrays_and_records => "arrays_and_records"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        deep_conditionals => "deep_conditionals"
        for_loops => "for_loops"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        lazy_operators => "lazy_operators"
        length => "length"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        operator_precedence => "operator_precedence"
        parse_minus => "parse_minus"
        real_comparisons => "real_comparisons"
        real_literals => "real_literals"
        records => "records"
        recursive_function => "recursive_function"
        recursive_types => "recursive_types"
        routine_redefinition => "routine_redefinition"
        shadow => "shadow"
        type_aliases => "type_aliases"
        type_conversions => "type_conversions"
        variable_declarations => "variable_declarations"
        while_loops => "while_loops"
    ]
    fail = [
        invalid => "invalid"
        lexer_invalid => "lexer_invalid"
    ]
}
