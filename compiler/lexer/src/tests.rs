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
        array_slice_eq => "array_slice_eq"
        arrays_and_records => "arrays_and_records"
        assert => "assert"
        binary_not => "binary_not"
        bool_arith => "bool_arith"
        bool_ord => "bool_ord"
        comparison_operators => "comparison_operators"
        complex_expressions => "complex_expressions"
        conditionals => "conditionals"
        constant => "constant"
        deep_conditionals => "deep_conditionals"
        default_init => "default_init"
        field_not_record => "field_not_record"
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
        malformed_integer => "malformed_integer"
        nested_control => "nested_control"
        new_array => "new_array"
        new_array_fixed => "new_array_fixed"
        no_such_field => "no_such_field"
        not_scalar_arith => "not_scalar_arith"
        not_scalar_ord => "not_scalar_ord"
        null_array_length => "null_array_length"
        null_record_field => "null_record_field"
        oob_big => "oob_big"
        oob_neg => "oob_neg"
        oob_zero => "oob_zero"
        operator_precedence => "operator_precedence"
        panic => "panic"
        parse_minus => "parse_minus"
        print => "print"
        raytracer => "raytracer"
        real_comparisons => "real_comparisons"
        real_literals => "real_literals"
        real_mod => "real_mod"
        records => "records"
        recursive_function => "recursive_function"
        recursive_types => "recursive_types"
        references => "references"
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
