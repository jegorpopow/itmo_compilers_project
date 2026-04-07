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
        assert => "assert"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        constant => "constant"
        deep_conditionals => "deep_conditionals"
        for_loops => "for_loops"
        function_parameters => "function_parameters"
        function_return => "function_return"
        identifiers => "identifiers"
        invalid => "invalid"
        invalid_array_length => "invalid_array_length"
        invalid_new_array_length => "invalid_new_array_length"
        lazy_operators => "lazy_operators"
        length => "length"
        lexer_invalid => "lexer_invalid"
        local_type => "local_type"
        logical_operators => "logical_operators"
        nested_control => "nested_control"
        new_array => "new_array"
        new_array_fixed => "new_array_fixed"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        operator_precedence => "operator_precedence"
        panic => "panic"
        parse_minus => "parse_minus"
        raytracer => "raytracer"
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
