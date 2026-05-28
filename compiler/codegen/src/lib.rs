use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

use ast::{
    AnalysisError, AnalysisResult, BinaryOperator, Binding, Bindings, Block, Decl, Expression,
    Interner, Literal, LvalueExpression, Program as AST, Representation, Routine, RoutineBody,
    RoutineDecl, Statement, Type, TypeId, UnaryOperator, VarDecl,
};
use common::{Identifier, Integer, Location, Position, RawIdentifier, Real, VarLoc};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    /// push int / bool onto stack
    IntConst {
        value: Integer,
    },
    /// push real onto stack
    RealConst {
        value: Real,
    },
    /// push null onto stack
    NullConst,
    /// push to stack
    Load {
        loc: Location,
    },
    /// pop from stack
    Store {
        loc: Location,
    },
    /// push address of variable to stack
    AddressOf {
        loc: Location,
    },
    /// duplicate stack top
    Dup,
    /// drop stack top
    Drop,
    // drop n elements from stack
    DropMany(VarLoc),
    /// swaps top and second elements of stack
    Swap,
    /// apply binary operator to stack top
    BinOp {
        op: BinaryOperator,
    },
    UnOp {
        op: UnaryOperator,
    },
    /// pop value and address from stack, write referenced value
    /// FIXME(Andrew Vlasenkov): ensure pop's order is correct in VM:
    /// value should be on top of a stack and address just below value
    StoreAddress,
    /// pop address from stack, read and push referenced value
    LoadAddress,
    /// allocate a record, push a reference to stack
    AllocRecord {
        type_id: TypeId,
        size: u64,
    },
    /// allocate an array, push a reference to stack
    AllocArray {
        type_id: TypeId,
        size: u64,
    }, // TODO: add TypeId ?
    AllocArrayDynamic {
        type_id: TypeId,
    },
    /// pop array ref from stack, push its size
    ArraySize, // TODO: add built-in function call
    /// pop element index and array ref from stack, push address of array[index]
    ElementAddress,
    /// pop record ref from stack, push its field address
    FieldAddress {
        field_offset: u64,
    },
    /// no-op
    Label {
        id: u64,
    },
    /// non-conditional jump
    Jump {
        label: u64,
    },
    /// conditional jump
    JumpZero {
        label: u64,
    },
    /// conditional jump
    JumpNotZero {
        label: u64,
    },
    /// leave function, the stack top is a return value
    Ret,
    /// call specified function
    Call {
        function_label: u64,
    },
    /// Print a stack top and drop it
    Print {
        type_id: TypeId,
    },
    /// Terminate program
    Panic {
        code: u64,
        line: u32,
        column: u16,
    },
    IntToBool, // All of it may be just a built-in call
    RealToInt, // All of it may be just a built-in call
    IntToReal, // All of it may be just a built-in call
}

#[derive(Debug)]
pub struct RTTI(pub Vec<Representation>);

#[derive(Debug)]
pub struct FunctionRecord {
    pub name: String,
    pub label_id: u64,
    pub args: Vec<TypeId>,
    pub result: TypeId,
}

#[derive(Debug)]
pub struct FunctionTable(pub Vec<FunctionRecord>);

#[derive(Debug)]
pub struct Program {
    pub global_count: usize,
    pub rtti: RTTI,
    pub function_table: FunctionTable,
    pub code: Vec<Instruction>,
}

#[derive(Debug)]
struct Compiler<'a> {
    bindings: &'a Bindings,
    interner: Interner,
    bytecode: Vec<Instruction>,
    global_init: Vec<Instruction>,
    fresh_label_counter: u64,
    routines_labels: BTreeMap<Identifier, u64>,
    routine_meta: HashMap<u64, (Vec<TypeId>, TypeId)>,
    global_count: usize,
}

impl<'a> Compiler<'a> {
    #[must_use]
    fn new(table: &'a Bindings) -> Self {
        let mut result = Compiler {
            bindings: table,
            interner: Interner::new(),
            bytecode: Vec::new(),
            global_init: Vec::new(),
            fresh_label_counter: 0,
            routines_labels: BTreeMap::new(),
            routine_meta: HashMap::new(),
            global_count: 0,
        };

        let global_init_label = result.get_fresh_label();
        assert_eq!(
            global_init_label, 0,
            "Global init section should have hardcoded label `0`"
        );

        result.global_init.push(Instruction::Label {
            id: global_init_label,
        });

        result
    }

    fn get_fresh_label(&mut self) -> u64 {
        let result = self.fresh_label_counter;
        self.fresh_label_counter += 1;
        result
    }

    fn compile_lvalue_expr(&mut self, expr: &LvalueExpression) -> AnalysisResult<()> {
        match expr {
            LvalueExpression::Identifier(identifier) => {
                self.bytecode.push(Instruction::AddressOf {
                    loc: self.bindings[identifier].ensure_is_var()?.relative_location,
                });
                Ok(())
            }
            LvalueExpression::Member {
                lhs, member_offset, ..
            } => {
                self.compile_lvalue_expr(lhs)?;
                self.bytecode.push(Instruction::FieldAddress {
                    field_offset: *member_offset,
                });
                Ok(())
            }
            LvalueExpression::Index { lhs, index } => {
                self.compile_expr(index)?;
                self.compile_lvalue_expr(lhs)?;
                self.bytecode.push(Instruction::ElementAddress);
                Ok(())
            }
        }
    }

    fn compile_new(
        &mut self,
        t: &Rc<Type>,
        fields: &[(RawIdentifier, Rc<Expression>)],
    ) -> AnalysisResult<()> {
        let effective_type = self.bindings.get_effective_type(t)?;

        match effective_type.as_ref() {
            Type::Int | Type::Real | Type::Bool | Type::Null => {
                return Err(AnalysisError {
                    what: "Unboxed types can not be new-constructed".to_string(),
                });
            }
            Type::Alias(_) => {
                return Err(AnalysisError {
                    what: "Effective type can not be alias".to_string(),
                });
            }
            Type::Record(record_description) => {
                self.bytecode.push(Instruction::AllocRecord {
                    type_id: self
                        .bindings
                        .get_type_representation(t, &mut self.interner)?,
                    size: record_description.fields.len() as u64,
                });

                for (name, expr) in fields {
                    let field_offset = record_description.get_field_index(name)?;
                    self.bytecode.push(Instruction::Dup);
                    self.bytecode
                        .push(Instruction::FieldAddress { field_offset });
                    self.compile_expr(expr)?;
                    self.bytecode.push(Instruction::StoreAddress);
                }
            }
            Type::Array(array_description) => {
                let Some(length) = array_description.length else {
                    unreachable!()
                };
                self.bytecode.push(Instruction::AllocArray {
                    type_id: self
                        .bindings
                        .get_type_representation(t, &mut self.interner)?,
                    size: length
                        .try_into()
                        .expect("Internal compiler error, too long array"),
                })
            }
        }
        Ok(())
    }

    fn compile_new_array(
        &mut self,
        element_type: &Rc<Type>,
        length: &Rc<Expression>,
    ) -> AnalysisResult<()> {
        self.compile_expr(length)?;
        self.bytecode.push(Instruction::AllocArrayDynamic {
            type_id: self
                .bindings
                .get_type_representation(element_type, &mut self.interner)?,
        });
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expression) -> AnalysisResult<()> {
        match expr {
            Expression::LvalueToRvalue(lvalue_expression) => match lvalue_expression.as_ref() {
                LvalueExpression::Identifier(identifier) => {
                    self.bytecode.push(Instruction::Load {
                        loc: self.bindings[identifier].ensure_is_var()?.relative_location,
                    });
                }
                LvalueExpression::Member { .. } | LvalueExpression::Index { .. } => {
                    self.compile_lvalue_expr(lvalue_expression)?;
                    self.bytecode.push(Instruction::LoadAddress);
                }
            },
            Expression::Literal(literal) => {
                self.bytecode.push(match *literal {
                    Literal::Bool { value } => Instruction::IntConst {
                        value: value.into(),
                    },
                    Literal::Integer { repr: _, value } => Instruction::IntConst { value },
                    Literal::Real { repr: _, value } => Instruction::RealConst { value },
                });
            }
            Expression::Call { callee, args } => {
                for arg in args {
                    self.compile_expr(arg)?;
                }
                let function_label = self.routines_labels.get(callee).ok_or(AnalysisError {
                    what: format!("Unknown function {callee}"),
                })?;
                self.bytecode.push(Instruction::Call {
                    function_label: *function_label,
                });
            }
            Expression::BinOp { op, lhs, rhs } => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                self.bytecode.push(Instruction::BinOp { op: *op });
            }
            Expression::UnOp { op, operand } => {
                self.compile_expr(operand)?;
                self.bytecode.push(Instruction::UnOp { op: *op });
            }
            Expression::Cast { operand, target: _ } => {
                self.compile_expr(operand)?;
            }
            Expression::New { t, fields } => {
                self.compile_new(t, fields.as_deref().unwrap_or_default())?
            }
            Expression::NewArray { elements, length } => {
                self.compile_new_array(elements, length)?;
            }
            Expression::LengthOf { arr } => {
                self.compile_expr(arr)?;
                self.bytecode.push(Instruction::ArraySize);
            }
            Expression::Null => {
                self.bytecode.push(Instruction::NullConst);
            }
            Expression::IntToBool(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::IntToBool);
            }
            Expression::BoolToInt(expression) => {
                self.compile_expr(expression)?;
            }
            Expression::RealToInt(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::RealToInt);
            }
            Expression::IntToReal(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::IntToReal);
            }
        }

        Ok(())
    }

    fn compile_block(&mut self, block: &Block) -> AnalysisResult<()> {
        for statement in &block.stmts {
            self.compile_statement(statement)?
        }

        // Discard local variables
        self.bytecode
            .push(Instruction::DropMany(block.locals_count));

        Ok(())
    }

    #[expect(clippy::too_many_lines, reason = "This lint is useless")]
    fn compile_statement(&mut self, stmt: &Statement) -> AnalysisResult<()> {
        match stmt {
            &Statement::Panic {
                pos: Position { line, column },
            } => self.bytecode.push(Instruction::Panic {
                code: 1,
                line: line.try_into().expect("Over 4 billion lines of code?"),
                column: column.try_into().expect(
                    "I understand choosing 120 for your line width instead of 80, \
                    but over 65 thousand characters in a single line? Really?",
                ),
            }),
            Statement::Assignment { lhs, rhs } => match lhs.as_ref() {
                LvalueExpression::Identifier(identifier) => {
                    // Microoptimisation: use direct Store instruction instead of calculating address
                    self.compile_expr(rhs)?;
                    self.bytecode.push(Instruction::Store {
                        loc: self.bindings[identifier].ensure_is_var()?.relative_location,
                    });
                }
                LvalueExpression::Index { .. } | LvalueExpression::Member { .. } => {
                    self.compile_lvalue_expr(lhs)?;
                    self.compile_expr(rhs)?;
                    self.bytecode.push(Instruction::StoreAddress);
                }
            },
            Statement::While { condition, body } => {
                let body_label = self.get_fresh_label();
                let condition_label = self.get_fresh_label();

                self.bytecode.push(Instruction::Jump {
                    label: condition_label,
                });
                self.bytecode.push(Instruction::Label { id: body_label });
                self.compile_block(body)?;
                self.bytecode.push(Instruction::Label {
                    id: condition_label,
                });
                self.compile_expr(condition)?;
                self.bytecode
                    .push(Instruction::JumpNotZero { label: body_label });
            }
            Statement::Expr(expression) => {
                self.compile_expr(expression)?;
                self.bytecode.push(Instruction::Drop);
            }
            Statement::If {
                condition,
                on_true,
                on_false,
            } => {
                let on_false_label = self.get_fresh_label();
                self.compile_expr(condition)?;
                self.bytecode.push(Instruction::JumpZero {
                    label: on_false_label,
                });
                self.compile_block(on_true)?;
                self.bytecode
                    .push(Instruction::Label { id: on_false_label });
                if let Some(on_false) = on_false {
                    self.compile_block(on_false)?;
                }
            }
            Statement::For {
                counter,
                lower_bound,
                upper_bound,
                order,
                body,
            } => {
                let body_label = self.get_fresh_label();
                let condition_label = self.get_fresh_label();

                self.compile_expr(lower_bound)?; // the current stack top is a counter location, so that line initialises the counter

                self.bytecode.push(Instruction::Jump {
                    label: condition_label,
                });
                self.bytecode.push(Instruction::Label { id: body_label });
                self.compile_block(body)?;

                let operator = match order {
                    common::LoopOrder::Direct => BinaryOperator::Int(ast::IntBinOp::Add),
                    common::LoopOrder::Reversed => BinaryOperator::Int(ast::IntBinOp::Sub),
                };

                let counter_expr = Rc::new(LvalueExpression::Identifier(counter.clone()));

                self.compile_statement(&Statement::Assignment {
                    lhs: Rc::clone(&counter_expr),
                    rhs: Expression::BinOp {
                        op: operator,
                        lhs: Expression::LvalueToRvalue(counter_expr).into(),
                        rhs: Expression::Literal(Literal::Integer {
                            repr: "1".to_string(),
                            value: 1,
                        })
                        .into(),
                    }
                    .into(),
                })?;

                self.bytecode.push(Instruction::Label {
                    id: condition_label,
                });

                self.bytecode.push(Instruction::Load {
                    loc: self.bindings[counter].ensure_is_var()?.relative_location,
                });
                self.compile_expr(upper_bound)?;

                match order {
                    common::LoopOrder::Direct => self.bytecode.push(Instruction::BinOp {
                        op: BinaryOperator::Int(ast::IntBinOp::Le),
                    }),
                    common::LoopOrder::Reversed => self.bytecode.push(Instruction::BinOp {
                        op: BinaryOperator::Int(ast::IntBinOp::Ge),
                    }),
                }
                self.bytecode
                    .push(Instruction::JumpNotZero { label: body_label });

                self.bytecode.push(Instruction::Drop);
            }
            Statement::ForEach {
                counter,
                index,
                collection,
                body,
                ..
            } => {
                let body_label = self.get_fresh_label();
                let condition_label = self.get_fresh_label();

                let counter_expr = Rc::new(LvalueExpression::Identifier(counter.clone()));
                let index_expr = Rc::new(LvalueExpression::Identifier(index.clone()));

                self.bytecode.push(Instruction::IntConst { value: 1 });
                self.bytecode.push(Instruction::NullConst);

                self.bytecode.push(Instruction::Jump {
                    label: condition_label,
                });

                self.bytecode.push(Instruction::Label { id: body_label });
                self.compile_statement(&Statement::Assignment {
                    lhs: counter_expr,
                    rhs: Expression::LvalueToRvalue(
                        LvalueExpression::Index {
                            lhs: Rc::clone(collection),
                            index: Expression::LvalueToRvalue(Rc::clone(&index_expr)).into(),
                        }
                        .into(),
                    )
                    .into(),
                })?;

                self.compile_block(body)?;

                self.compile_statement(&Statement::Assignment {
                    lhs: Rc::clone(&index_expr),
                    rhs: Expression::BinOp {
                        op: BinaryOperator::Int(ast::IntBinOp::Add),
                        lhs: Expression::LvalueToRvalue(Rc::clone(&index_expr)).into(),
                        rhs: Expression::Literal(Literal::Integer {
                            repr: "1".to_string(),
                            value: 1,
                        })
                        .into(),
                    }
                    .into(),
                })?;

                self.bytecode.push(Instruction::Label {
                    id: condition_label,
                });

                self.compile_expr(&Expression::BinOp {
                    op: BinaryOperator::Int(ast::IntBinOp::Le),
                    lhs: Expression::LvalueToRvalue(index_expr).into(),
                    rhs: Expression::LengthOf {
                        arr: Expression::LvalueToRvalue(Rc::clone(collection)).into(),
                    }
                    .into(),
                })?;

                self.bytecode
                    .push(Instruction::JumpNotZero { label: body_label });

                self.bytecode.push(Instruction::DropMany(2));
            }
            Statement::Print { value, t } => {
                self.compile_expr(value)?;
                self.bytecode.push(Instruction::Print {
                    type_id: self
                        .bindings
                        .get_type_representation(t, &mut self.interner)?,
                });
            }
            Statement::Return { value } => {
                self.compile_expr(value)?;
                self.bytecode.push(Instruction::Ret);
            }

            Statement::Declaration(Binding { name: _, decl }) => match decl {
                ast::LocalDecl::Var(VarDecl {
                    t,
                    initialiser,
                    relative_location: _,
                }) => {
                    let initialiser = match initialiser {
                        Some(initialiser) => initialiser.as_ref(),
                        None => &self.bindings.get_default_initialiser(t)?,
                    };

                    // Local variable initialisation is just a `push`
                    self.compile_expr(initialiser)?;
                }

                ast::LocalDecl::Type(_) | ast::LocalDecl::Const(_) => (),
            },
        }

        Ok(())
    }

    fn collect_routines(&mut self, program: &AST) {
        for binding in &program.globals {
            if let Decl::Routine(_) = &binding.decl {
                let fresh = self.get_fresh_label();
                let _: Option<u64> = self.routines_labels.insert(binding.name.clone(), fresh);
            }
        }
    }

    fn compile(&mut self, program: &AST) -> AnalysisResult<()> {
        self.global_count = program
            .globals
            .iter()
            .filter(|b| matches!(b.decl, Decl::Var(_)))
            .count();

        for Binding { name, decl } in &program.globals {
            match decl {
                Decl::Var(v) => {
                    // Some dirty hacks below
                    let bytecode = std::mem::take(&mut self.bytecode);

                    let initialiser = match &v.initialiser {
                        Some(expr) => Rc::clone(expr),
                        None => Rc::new(self.bindings.get_default_initialiser(&v.t)?),
                    };

                    self.compile_statement(&Statement::Assignment {
                        lhs: LvalueExpression::Identifier(name.clone()).into(),
                        rhs: initialiser,
                    })?;

                    let mut global_init = std::mem::take(&mut self.bytecode);
                    self.bytecode = bytecode;
                    self.global_init.append(&mut global_init);
                }
                Decl::Routine(r) => match r {
                    RoutineDecl::Forward { .. } => {}
                    RoutineDecl::Full(Routine {
                        body, signature, ..
                    }) => {
                        let label_id = *self
                            .routines_labels
                            .get(name)
                            .expect("Routines are indexed");
                        self.bytecode.push(Instruction::Label { id: label_id });

                        let arg_type_ids: Vec<TypeId> = signature
                            .args
                            .iter()
                            .map(|(_, t)| {
                                self.bindings.get_type_representation(t, &mut self.interner)
                            })
                            .collect::<Result<_, _>>()?;
                        let return_type_id = self
                            .bindings
                            .get_type_representation(&signature.return_type, &mut self.interner)?;
                        let prev = self
                            .routine_meta
                            .insert(label_id, (arg_type_ids, return_type_id));
                        debug_assert_eq!(
                            prev, None,
                            "compiler bug: multiple bodies for label {label_id}"
                        );

                        match body {
                            RoutineBody::Block(block) => {
                                self.compile_block(block)?;
                                self.bytecode.push(Instruction::NullConst);
                                self.bytecode.push(Instruction::Ret);
                            }
                            RoutineBody::Expression(expression) => {
                                self.compile_expr(expression)?;
                                self.bytecode.push(Instruction::Ret);
                            }
                        }
                    }
                },
                Decl::Const(_) | Decl::Type(_) => {}
            }
        }
        Ok(())
    }
}

impl From<Compiler<'_>> for Program {
    fn from(value: Compiler<'_>) -> Self {
        let Compiler {
            bindings: _,
            interner,
            bytecode,
            global_init,
            fresh_label_counter: _,
            routines_labels,
            routine_meta,
            global_count,
        } = value;
        let mut code = global_init;
        if let Some(&main_label) = routines_labels
            .iter()
            .find(|(ident, _)| ident.raw.name == "main")
            .map(|(_, label)| label)
        {
            code.push(Instruction::Call {
                function_label: main_label,
            });
            code.push(Instruction::Drop);
        }
        code.push(Instruction::NullConst);
        code.push(Instruction::Ret);

        code.extend(bytecode);
        let function_table = FunctionTable(
            routines_labels
                .into_iter()
                .map(|(name, label_id)| {
                    let (args, result) = routine_meta
                        .get(&label_id)
                        .cloned()
                        .unwrap_or_else(|| (Vec::new(), TypeId(0)));
                    FunctionRecord {
                        name: name.raw.name.clone(),
                        label_id,
                        args,
                        result,
                    }
                })
                .collect(),
        );
        Program {
            code,
            rtti: RTTI(interner.to_table()),
            function_table,
            global_count,
        }
    }
}

pub fn compile(program: &AST) -> AnalysisResult<Program> {
    let mut compiler = Compiler::new(&program.bindings);
    compiler.collect_routines(program);
    compiler.compile(program)?;
    Ok(compiler.into())
}
