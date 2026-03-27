use crate::Lexer;

fn lex(src: &str) -> Result<String, core::fmt::Error> {
    let mut result = String::new();
    for token in Lexer::from(src) {
        use core::fmt::Write;
        writeln!(&mut result, "{token}")?
    }
    Ok(result)
}

testing::tests! {
    folder = "lexer"
    extension = "txt"
    fun = lex
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
        invalid => "invalid"
        lazy_operators => "lazy_operators"
        length => "length"
        lexer_invalid => "lexer_invalid"
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
    ]
}
