use std::{collections::HashMap, rc::Rc};

use ast::{
    AnalysisError, AnalysisResult, Binding, Bindings, Block, Decl, Expression, Interner, Literal,
    LvalueExpression, Program, Routine, RoutineBody, RoutineDecl, Statement, Type, TypeId, VarDecl,
};
use common::{Identifier, Integer, Position, RawIdentifier};

pub mod bytecode;

use bytecode::Instruction;

use crate::bytecode::{BytecodeFile, FunctionRecord, FunctionTable, RTTI};

#[derive(Debug)]
pub struct Compiler<'a> {
    bindings: &'a Bindings,
    interner: Interner,
    bytecode: Vec<Instruction>,
    fresh_label_counter: u64,
    routines_labels: HashMap<Identifier, u64>,
}

impl<'a> Compiler<'a> {
    #[must_use]
    pub fn new(table: &'a Bindings) -> Self {
        Compiler {
            bindings: table,
            interner: Interner::new(),
            bytecode: Vec::new(),
            fresh_label_counter: 0,
            routines_labels: HashMap::new(),
        }
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
                    size: 0,
                });

                // Fields initialisation
                for (name, expr) in fields {
                    self.bytecode.push(Instruction::Dup);
                    self.bytecode.push(Instruction::IntConst {
                        value: Integer::try_from(record_description.get_field_index(name)?)
                            .expect("Internal compiler error: to big structure type"),
                    });
                    self.bytecode.push(Instruction::ElementAddress);
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
                for arg in args.iter().rev() {
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
                    common::LoopOrder::Direct => ast::BinaryOperator::Int(ast::IntBinOp::Add),
                    common::LoopOrder::Reversed => ast::BinaryOperator::Int(ast::IntBinOp::Sub),
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

                self.compile_expr(upper_bound)?; // Recalculating upper bound in case of its changing

                match order {
                    common::LoopOrder::Direct => self.bytecode.push(Instruction::BinOp {
                        op: ast::BinaryOperator::Int(ast::IntBinOp::Le),
                    }),
                    common::LoopOrder::Reversed => self.bytecode.push(Instruction::BinOp {
                        op: ast::BinaryOperator::Int(ast::IntBinOp::Ge),
                    }),
                }
                self.bytecode.push(Instruction::Swap);
                self.bytecode.push(Instruction::Drop);
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
                        op: ast::BinaryOperator::Int(ast::IntBinOp::Add),
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
                    op: ast::BinaryOperator::Int(ast::IntBinOp::Le),
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

    pub fn collect_routines(&mut self, program: &Program) {
        for binding in &program.globals {
            if let Decl::Routine(_) = &binding.decl {
                let fresh = self.get_fresh_label();
                let _: Option<u64> = self.routines_labels.insert(binding.name.clone(), fresh);
            }
        }
    }

    pub fn compile(&mut self, program: &Program) -> AnalysisResult<()> {
        for Binding { name, decl } in &program.globals {
            match decl {
                Decl::Var(v) => todo!("var {name:?} = {v:?}"),
                Decl::Const(v) => todo!("const {name:?} = {v:?}"),
                Decl::Type(t) => todo!("type {name:?} = {t:?}"),
                Decl::Routine(r) => match r {
                    RoutineDecl::Forward { .. } => todo!(),
                    RoutineDecl::Full(Routine {
                        signature,
                        args_bindings,
                        body,
                    }) => {
                        match body {
                            RoutineBody::Block(block) => self.compile_block(block)?,

                            RoutineBody::Expression(expression) => self.compile_expr(expression)?,
                        }
                        todo!(
                            "Do something with signature {signature:?} and args {args_bindings:?}"
                        )
                    }
                },
            }
        }
        Ok(())
    }

    #[expect(clippy::wrong_self_convention, reason = "The convention is stupid")]
    fn to_bytecode_file(self) -> BytecodeFile {
        let instructions = self.bytecode;

        let function_table = FunctionTable(
            self.routines_labels
                .iter()
                .map(|(name, label)| FunctionRecord {
                    name: name.raw.name.clone(),
                    label_id: *label,
                    args: Vec::new(),
                    result: TypeId(0),
                })
                .collect(),
        );
        BytecodeFile {
            code: instructions,
            rtti: RTTI(self.interner.to_table()),
            function_table,
        }
    }
}

pub fn codegen(program: &Program) -> AnalysisResult<BytecodeFile> {
    let mut compiler = Compiler::new(&program.bindings);
    compiler.compile(program)?;
    Ok(compiler.to_bytecode_file())
}
