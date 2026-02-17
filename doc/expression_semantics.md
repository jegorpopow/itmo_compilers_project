# Expression Semantics - Clarifications

This document clarifies unclear points in the language specification regarding expression evaluation.

## Type Conversions

### Conversions in Assignments

According to the specification, type conversions occur during assignments. The conversion rules are:

**To `integer`:**
- `integer` ← `integer`: direct value copying
- `integer` ← `real`: **rounding to nearest integer** (not truncation)
- `integer` ← `boolean`: `true` → `1`, `false` → `0`

**To `real`:**
- `real` ← `real`: direct value copying
- `real` ← `integer`: direct value copying (no conversion)
- `real` ← `boolean`: `true` → `1.0`, `false` → `0.0`

**To `boolean`:**
- `boolean` ← `boolean`: direct value copying
- `boolean` ← `integer`: `1` → `true`, `0` → `false`; **runtime error** for other values
- `boolean` ← `real`: **illegal** (assignment error)

**Note:** The same conversion rules apply when passing arguments to routines, as parameter passing has the same semantics as assignment.

### Implicit Conversions in Arithmetic

When mixing `integer` and `real` in arithmetic operations, `integer` is automatically promoted to `real`:

- `int + real` → `real`
- `real + int` → `real`
- `int - real` → `real`
- `real * int` → `real`
- etc.

The result type is always `real` when mixing types.

## Logical Operators

### Lazy Evaluation (Short-Circuit)

The `and` and `or` operators use **lazy evaluation** (short-circuit semantics):

- **`e1 and e2`**: 
  - If `e1` evaluates to `false`, `e2` is **not evaluated** and result is `false`
  - If `e1` is `true`, evaluate `e2` and return its value

- **`e1 or e2`**:
  - If `e1` evaluates to `true`, `e2` is **not evaluated** and result is `true`
  - If `e1` is `false`, evaluate `e2` and return its value

- **`e1 xor e2`**: 
  - **Not lazy** - both operands are always evaluated
  - Returns `true` if operands differ, `false` if they're the same

### Logical Negation (`not`) - Specification Extension

The `not` operator is defined in the grammar as part of `Primary`, but its semantics are not fully specified. This section clarifies its behavior as an extension to the specification and treats `not` as a logical operator.

**Syntax:** The `not` operator can be applied to literals and expressions:
```
Primary : [ Sign | not ] IntegerLiteral | [ Sign ] RealLiteral | true | false | ModifiablePrimary | RoutineCall
```

**Semantics by operand type:**

- **`not boolean`**: Logical negation
  - `not true` → `false`
  - `not false` → `true`
  - Result type: `boolean`

- **`not integer`**: Converts integer to boolean, negates, converts back to integer
  - Conversion rule: `0` → `false`, any non-zero value → `true`
  - `not 0` → `1` (false → true → 1)
  - `not 1` → `0` (true → false → 0)
  - `not 5` → `0` (non-zero treated as true, negated to false → 0)
  - `not -3` → `0` (non-zero treated as true, negated to false → 0)
  - Result type: `integer`

- **`not real`**: Not supported (would require boolean conversion which is illegal for `real`)

**Important notes:**

1. The `not` operator on integers uses a **permissive conversion** where any non-zero integer is treated as `true`. This differs from the strict assignment conversion rule (`boolean` ← `integer`) which only accepts `0` or `1` and causes a runtime error for other values.

2. The `not` operator can be applied to expressions, not just literals, as it appears in the `Primary` grammar rule which includes `ModifiablePrimary` and `RoutineCall`.

3. Although it is syntactically unary, semantically `not` belongs to logical operators.

## Reference Equality

The `=` and `/=` operators for reference types (records, arrays) perform **identity comparison** (pointer equality), not structural equality:

- Two references are equal only if they point to the **same object** in memory
- Two records with identical field values are **not equal** unless they are the same object

**Note:** The specification uses `=` for equality and `/=` for inequality (not `==` and `!=`).

## Unary Operators

### Negation (`-`)

- `-int` → integer negation
- `-real` → real negation

## Operator Precedence

According to the grammar, operator precedence (from highest to lowest) is:

1. **Primary** (literals, variables, routine calls, member access `.`, indexing `[]`)
2. **Unary arithmetic operators** (`+`, `-`) - applied to Primary
3. **Multiplicative** (`*`, `/`, `%`) - left-associative
4. **Additive** (`+`, `-`) - left-associative  
5. **Comparison** (`<`, `<=`, `>`, `>=`, `=`, `/=`) - binary operators
6. **Logical** (`not`, `and`, `or`, `xor`) - left-associative, with `not` binding stronger than binary logical operators

The grammar structure:
```
Expression : Relation { ( and | or | xor ) Relation }
Relation : Simple [ ( < | <= | > | >= | = | /= ) Simple ]
Simple : Factor { ( * | / | % ) Factor }
Factor : Summand { ( + | - ) Summand }
Summand : Primary | ( Expression )
Primary : [ Sign | not ] IntegerLiteral | [ Sign ] RealLiteral | true | false | ModifiablePrimary | RoutineCall
```

## Order of Evaluation

**Left-to-right evaluation order**:

- **Binary operators**: For `e1 op e2`, `e1` is evaluated first, then `e2`.
- **Function calls**: For `f(e1, e2, ..., en)`, arguments are evaluated left-to-right (`e1`, then `e2`, ..., then `en`). The function name `f` is just an identifier lookup (not an expression to evaluate).
- **Member access**: For `e.m`, `e` is evaluated first (to get the record/object), then the member `m` is accessed by name (no evaluation needed for the member name).
- **Array indexing**: For `e[i]`, `e` is evaluated first (to get the array), then the index expression `i` is evaluated.
- **No reordering**: The compiler must not reorder evaluation of subexpressions with side effects. Each subexpression is evaluated exactly once.

**Interaction with lazy operators:**
- In `e1 and e2`: `e1` is always evaluated first. If `e1` is `false`, `e2` is not evaluated.
- In `e1 or e2`: `e1` is always evaluated first. If `e1` is `true`, `e2` is not evaluated.
- In `e1 xor e2`: Both `e1` and `e2` are always evaluated, left-to-right.

## Integer Division and Modulo

**Integer division** (`/` for `integer` operands):
- Truncates **toward zero**
- `7 / 3` → `2`
- `-7 / 3` → `-2`
- `7 / -3` → `-2`

**Integer modulo** (`%` for `integer` operands):
- Result has the **same sign as the dividend** (left operand)
- Satisfies: `a = (a / b) * b + (a % b)`
- `7 % 3` → `1`
- `-7 % 3` → `-1`
- `7 % -3` → `1`
- `-7 % -3` → `-1`

**Real division** (`/` for `real` operands):
- Standard floating-point division
- No special truncation rules

## Integer Overflow

**Overflow behavior** (implementation-defined):
- Integer arithmetic operations that exceed the implementation's range result in **wraparound** (2's complement behavior) or **implementation-defined behavior**.
- The language does not guarantee detection of overflow.
- Examples: `max_int + 1` may wrap to `min_int` or have implementation-defined behavior.

**Range limits:**
- The specification does not define exact ranges for `integer` and `real` types.
- Each implementation defines its own limits (e.g., 32-bit or 64-bit integers).

## Cross-Type Comparisons

**Equality operators** (`=`, `/=`):
- **Same type**: Direct comparison
- **`integer` vs `real`**: Both operands are promoted to `real`, then compared
  - `1 = 1.0` → `true`
  - `1 /= 1.0` → `false`
- **`boolean`**: Can only be compared with `boolean` (no cross-type equality)
- **Reference types**: Identity comparison (same object in memory)

**Ordering operators** (`<`, `<=`, `>`, `>=`):
- **Same numeric type**: Direct comparison
- **`integer` vs `real`**: Both operands promoted to `real`, then compared
  - `1 < 2.5` → `true`
  - `3.0 <= 3` → `true`
- **`boolean`**: Cannot be used with ordering operators (type error)

## Floating-Point Edge Cases

**Division by zero for `real`:**
- `real / 0.0` causes a **runtime error** (not infinity)
- The language does not support IEEE-754 infinity or NaN values

**Real-to-integer conversion:**
- Uses **rounding to nearest integer** (as specified in assignment conversions)
- `3.7` → `4`, `3.2` → `3`, `-2.5` → `-2` or `-3` (implementation-defined for exactly `.5`)

## Conditions and Truthiness

**Boolean conditions:**
- In `if Expression then` and `while Expression loop`, the expression must evaluate to a type that can be used as a condition.

**Type rules for conditions:**
- **`boolean`**: Direct use (`true`/`false`)
- **`integer`**: Treated as boolean (C-like truthiness)
  - `0` → `false`
  - Any non-zero value → `true`
  - Example: `if 5 then ...` is valid and executes the `then` branch
- **`real`**: Cannot be used as condition (type error)

**Note:** This truthiness rule applies only to control flow conditions (`if`, `while`). For assignments, the strict conversion rule applies (`boolean` ← `integer` only accepts `0` or `1`).

## Function Call Semantics

**Function calls as expressions:**
- A routine call `f(...)` is an **expression** if the routine has a return type (declared with `: Type`).
- A routine call without a return type is only valid as a **statement** (discards any implicit return value).

**Return value requirement:**
- If a function (routine with return type) does not execute a `return` statement before reaching the end of its body, the behavior is **undefined** or causes a runtime error (implementation-defined).

**Function calls in expressions:**
- Function calls can appear in any expression position:
  - As operands: `f() + g()`
  - In array indices: `arr[f()]`
  - In record fields: `rec.field(f())`
  - As arguments: `h(f(), g())`

## Error Conditions

The following cause **runtime errors** that terminate program execution:

- **Division by zero**: `int / 0` or `real / 0.0`
- **Modulo by zero**: `int % 0`
- **Invalid `boolean` ← `integer` conversion**: integer value not in `{0, 1}`
- **Illegal assignment**: `boolean` ← `real`
- **Array index out of bounds**: accessing `arr[i]` where `i < 0` or `i > arr.length`
- **Null reference access**: accessing members or indices of an uninitialized reference (if applicable)
- **Function without return**: function with return type reaches end without `return` (implementation-defined)

**Error model:**
- When a runtime error occurs during expression evaluation, the program **terminates immediately**.
- Any side effects that occurred before the error (e.g., assignments, function calls) **remain in effect**.
- There is no exception handling mechanism.

## Constant Expressions

**Definition:**
- A **constant expression** is an expression that can be evaluated at compile time.
- Constant expressions are required for array size declarations: `array [ Expression ] Type`

**Restrictions for constant expressions:**
- Must not contain variable references
- Must not contain function calls
- Must not contain operations with side effects
- Can contain: literals, arithmetic operations on literals, type conversions of literals

**Examples of constant expressions:**
- `5`, `3.14`, `true`
- `2 + 3`, `10 * 5`
- `(2 + 3) * 4`

**Examples of non-constant expressions:**
- `x + 5` (contains variable)
- `f(5)` (contains function call)
- `arr[1]` (contains array access)

## Notes

- **Type aliases**: When a type alias is declared (e.g., `type kilometers is real`), it creates a distinct type that is incompatible with its underlying type for assignments. Both sides of an assignment must have the same type name.
- **Array length**: Arrays may have a `.length` field accessible via dotted notation (implementation detail).

