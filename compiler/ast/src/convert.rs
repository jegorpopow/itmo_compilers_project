use core::iter;
use std::{collections::HashMap, rc::Rc};

use common::{BindingId, Identifier, Location, LoopOrder, RawIdentifier, VarLoc};

use crate::{
    operators::UnaryOperator,
    tree::{ConstDecl, cast_to},
    types::{BinOpAdjustment, infer_binary_operator_type},
    *,
};

#[derive(Debug, Default)]
struct Scope {
    binders: HashMap<RawIdentifier, BindingId>,
    locals_in_block: VarLoc,
}

impl Scope {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    fn lookup(&self, name: &RawIdentifier) -> Option<BindingId> {
        self.binders.get(name).copied()
    }

    fn add(&mut self, ident: &Identifier) {
        let Identifier { raw, id } = ident;
        if let Some(prev) = self.binders.insert(raw.to_owned(), *id) {
            unreachable!("Duplicate bindings for the same ident ({raw:?}): {prev} & {id}")
        }
    }
}

#[derive(Debug)]
struct RoutinePrototype {
    #[expect(dead_code, reason = "FIXME")]
    name: RawIdentifier,
    #[expect(dead_code, reason = "FIXME")]
    args: Vec<Rc<Type>>,
    return_type: Rc<Type>,
}

#[derive(Debug, Default)]
struct Converter {
    bindings: Bindings,
    global_scope: Scope,
    local_scopes: Vec<Scope>,
    global_count: VarLoc,
    local_count: VarLoc,
    current_routine: Option<RoutinePrototype>,
}

impl Converter {
    fn new() -> Self {
        Default::default()
    }

    fn scoped<T>(&mut self, callback: impl FnOnce(&mut Self) -> T) -> (T, VarLoc) {
        self.local_scopes.push(Scope::new());

        let result = callback(self);

        let Scope {
            binders: _,
            locals_in_block,
        } = self
            .local_scopes
            .pop()
            .expect("Scopes should not be pushed/popped outside of this method");

        self.local_count -= locals_in_block;
        (result, locals_in_block)
    }

    fn get_fresh_global_location(&mut self) -> Location {
        let res = self.global_count;
        self.global_count += 1;
        Location::Global(res)
    }

    fn get_fresh_local_location(&mut self) -> Location {
        let res = self.local_count;
        self.local_count += 1;
        self.local_scopes
            .last_mut()
            .expect("Cannot get a local location outside of any local scopes")
            .locals_in_block += 1;
        Location::Local(res)
    }

    fn bind_global_decl(&mut self, name: &RawIdentifier, decl: Decl) -> Identifier {
        // TODO: process function forward declaration
        let ident = self.bindings.create(name, decl);
        self.global_scope.add(&ident);
        ident
    }

    fn rebind_decl(&mut self, ident: &Identifier, new_decl: Decl) {
        self.bindings.rebind(ident, new_decl);
    }

    fn bind_local_decl(&mut self, name: &RawIdentifier, decl: LocalDecl) -> Identifier {
        let ident = self.bindings.create(name, decl.into());
        self.local_scopes
            .last_mut()
            .expect("Cannot bind a local decl outside of any local scopes")
            .add(&ident);

        ident
    }

    fn bind_routine(
        &mut self,
        routine_name: &RawIdentifier,
        decl: RoutineDecl,
    ) -> AnalysisResult<Identifier> {
        let existing_binding = self.lookup(routine_name);

        Ok(match existing_binding {
            Ok(Binding {
                name: ident,
                decl: Decl::Routine(existing_decl),
            }) => match [existing_decl, &decl] {
                [_, _] if existing_decl.signature() != decl.signature() => Err(AnalysisError {
                    what: format!(
                        "Conflicting signature for declarations of routine {routine_name:?}"
                    ),
                })?,

                [RoutineDecl::Full(..), RoutineDecl::Full { .. }] => Err(AnalysisError {
                    what: format!("Conflicting declarations of routine {routine_name:?}"),
                })?,

                [RoutineDecl::Forward { .. }, RoutineDecl::Full { .. }] => {
                    let ident = ident.to_owned();
                    self.rebind_decl(&ident, Decl::Routine(decl));
                    ident
                }

                [
                    RoutineDecl::Full(..) | RoutineDecl::Forward { .. },
                    RoutineDecl::Forward { .. },
                ] => ident.to_owned(),
            },

            Ok(Binding {
                name: _,
                decl: Decl::Type(_) | Decl::Var(_) | Decl::Const(_),
            })
            | Err(_) => {
                // function just shadows previous global variable or type with the same name
                self.bind_global_decl(routine_name, Decl::Routine(decl))
            }
        })
    }

    fn lookup<'a>(&'a self, name: &RawIdentifier) -> AnalysisResult<&'a Binding> {
        self.local_scopes
            .iter()
            .rev()
            .chain(iter::once(&self.global_scope))
            .find_map(|scope_block| scope_block.lookup(name))
            .map(|id| &self.bindings[id])
            .ok_or(AnalysisError {
                what: format!("Unknown name `{name}`"),
            })
    }

    fn convert_type(&self, t: &parser::Type) -> AnalysisResult<Rc<Type>> {
        Ok(match t {
            parser::Type::Int => Rc::new(Type::Int),
            parser::Type::Real => Rc::new(Type::Real),
            parser::Type::Bool => Rc::new(Type::Bool),
            parser::Type::Alias(raw_identifier) => {
                let decl = self.lookup(raw_identifier)?;
                let _: &_ = decl.ensure_is_type()?;
                Rc::new(Type::Alias(decl.name.clone()))
            }
            parser::Type::Record(parser::RecordDescription { fields }) => {
                let fields: Vec<FieldDescription> = fields
                    .iter()
                    .map(|parser::FieldDescription { name, t }| {
                        self.convert_type(t).map(|t| FieldDescription {
                            name: name.clone(),
                            t,
                        })
                    })
                    .collect::<AnalysisResult<_>>()?;
                Rc::new(Type::Record(RecordDescription { fields }))
            }
            parser::Type::Array(parser::ArrayDescription { t, length }) => {
                let element_type = self.convert_type(t)?;

                match length {
                    Some(expr) => {
                        let Typed { value, ty: expr_ty } = self.convert_expr(expr)?;
                        expr_ty.ensure_is(&Type::Int)?;
                        Rc::new(Type::Array(ArrayDescription {
                            t: element_type,
                            length: Some(value.try_constexpr_evaluate()?.as_usize()?),
                        }))
                    }
                    None => Rc::new(Type::Array(ArrayDescription {
                        t: element_type,
                        length: None,
                    })),
                }
            }
        })
    }

    fn convert_lvalue_expr(
        &self,
        tree: &parser::LvalueExpression,
    ) -> AnalysisResult<Typed<LvalueExpression>> {
        Ok(match tree {
            parser::LvalueExpression::Identifier(raw_identifier) => {
                match self.lookup(raw_identifier)? {
                    Binding {
                        name,
                        decl: Decl::Var(VarDecl { t, .. }),
                    } => Typed {
                        value: Rc::new(LvalueExpression::Identifier(name.clone())),
                        ty: Rc::clone(t),
                    },
                    t => Err(AnalysisError {
                        what: format!(
                            "{:?} is not a name of variable in lvalue expression",
                            t.name
                        ),
                    })?,
                }
            }
            parser::LvalueExpression::Member { lhs, member_name } => {
                let Typed { value: lhs, ty: t } = self.convert_lvalue_expr(lhs)?;
                Typed {
                    value: Rc::new(LvalueExpression::Member {
                        lhs,
                        member_name: member_name.clone(),
                    }),
                    ty: self
                        .bindings
                        .get_effective_type(&t)?
                        .get_field_type(member_name)?,
                }
            }
            parser::LvalueExpression::Index { lhs, index } => {
                let Typed {
                    value: lhs,
                    ty: lhs_t,
                } = self.convert_lvalue_expr(lhs)?;
                let effective_lhs_type = self.bindings.get_effective_type(&lhs_t)?;
                let Typed {
                    value: index,
                    ty: rhs_t,
                } = self.convert_expr(index)?;
                rhs_t.ensure_is(&Type::Int)?;
                Typed {
                    value: Rc::new(LvalueExpression::Index { lhs, index }),
                    ty: Rc::clone(effective_lhs_type.get_element_type()?),
                }
            }
        })
    }

    fn convert_unary(op: parser::Operator, operand: Typed) -> AnalysisResult<Typed> {
        let Typed {
            value: operand,
            ty: operand_type,
        } = operand;

        Ok(match op {
            parser::Operator::Not => match &*operand_type {
                Type::Bool => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::BoolNeg,
                        operand,
                    }),
                    ty: Rc::new(Type::Bool),
                },
                Type::Int => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::BoolNeg,
                        operand: Rc::new(Expression::IntToBool(operand)),
                    }),
                    ty: Rc::new(Type::Bool),
                },

                Type::Real
                | Type::Alias(_)
                | Type::Record(_)
                | Type::Array(_)
                | Type::Null
                | Type::Unit => Err(AnalysisError {
                    what: format!(
                        "Logical negation operator can not be applied to non-boolean {operand_type:?} value"
                    ),
                })?,
            },
            parser::Operator::Minus => match &*operand_type {
                Type::Int => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::IntNeg,
                        operand,
                    }),
                    ty: Rc::new(Type::Int),
                },
                Type::Real => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::RealNeg,
                        operand,
                    }),
                    ty: Rc::new(Type::Real),
                },
                Type::Bool
                | Type::Alias(_)
                | Type::Record(_)
                | Type::Array(_)
                | Type::Null
                | Type::Unit => Err(AnalysisError {
                    what: format!(
                        "Arithmetical negation operator can not be applied to non-scalar type {operand_type:?} value"
                    ),
                })?,
            },
            parser::Operator::Plus
            | parser::Operator::Mul
            | parser::Operator::Div
            | parser::Operator::Mod
            | parser::Operator::Eq
            | parser::Operator::Ne
            | parser::Operator::Lt
            | parser::Operator::Le
            | parser::Operator::Gt
            | parser::Operator::Ge
            | parser::Operator::And
            | parser::Operator::Or
            | parser::Operator::Xor => Err(AnalysisError {
                what: format!("Operator {op:?} can not be applied as unary"),
            })?,
        })
    }

    fn convert_call(
        &self,
        callee: &RawIdentifier,
        args: &[Rc<parser::Expression>],
    ) -> AnalysisResult<Typed> {
        let RoutineSignature {
            args: formal_args,
            return_type,
        } = self.lookup(callee)?.ensure_is_routine()?.signature();
        let arguments_types: Vec<Rc<Type>> = formal_args
            .iter()
            .map(|(_, arg_type)| Rc::clone(arg_type))
            .collect();

        let args: Vec<Typed> = args
            .iter()
            .map(|arg| self.convert_expr(arg))
            .collect::<AnalysisResult<_>>()?;

        if arguments_types.len() != args.len() {
            Err(AnalysisError {
                what: format!(
                    "routine `{callee}` expects {} arguments, but got {}",
                    arguments_types.len(),
                    args.len()
                ),
            })?
        }

        Ok(Typed {
            value: Rc::new(Expression::Call {
                callee: self.lookup(callee)?.name.clone(),
                args: iter::zip(arguments_types, args)
                    .map(|(arg_type, expr)| cast_to(expr, &arg_type))
                    .collect::<AnalysisResult<_>>()?,
            }),
            ty: Rc::clone(return_type),
        })
    }

    fn convert_new(
        &self,
        t: &parser::Type,
        fields: Option<&[(RawIdentifier, Rc<parser::Expression>)]>,
        array_length: Option<&parser::Expression>,
    ) -> AnalysisResult<Typed> {
        let ty = self.convert_type(t)?;
        let effective = self.bindings.get_effective_type(&ty)?;

        if let Some(length) = array_length {
            let Type::Array(ArrayDescription {
                t: elements,
                length: None,
            }) = Rc::unwrap_or_clone(effective)
            else {
                return Err(AnalysisError {
                    what: "new[] is only supported for array[] types without length".to_string(),
                });
            };
            return Ok(Typed {
                value: Rc::new(Expression::NewArray {
                    elements,
                    length: cast_to(self.convert_expr(length)?, &Type::Int)?,
                }),
                ty,
            });
        }

        match effective.as_ref() {
            Type::Int | Type::Real | Type::Bool | Type::Null | Type::Unit => Err(AnalysisError {
                what: format!("No new operator supported for built-in type {t:?}"),
            })?,
            Type::Alias(_) => unreachable!("Effective type can not be alias"),
            Type::Record(record) => Ok(Typed {
                value: Rc::new(Expression::New {
                    t: Rc::clone(&ty),
                    fields: Some(
                        fields
                            .unwrap_or_default()
                            .iter()
                            .map(|(name, expr)| {
                                self.convert_expr(expr).and_then(|expr| {
                                    Ok((
                                        name.clone(),
                                        cast_to(expr, record.get_field_type(name)?.as_ref())?,
                                    ))
                                })
                            })
                            .collect::<AnalysisResult<_>>()?,
                    ),
                }),
                ty,
            }),
            Type::Array(_) if fields.is_some() => Err(AnalysisError {
                what: format!("No field initialization possible for array type {t:?}"),
            }),

            Type::Array(ArrayDescription { length: None, t: _ }) => Err(AnalysisError {
                what: format!("No new length known array creation {t:?}"),
            }),

            Type::Array(ArrayDescription {
                length: Some(_),
                t: _,
            }) => Ok(Typed {
                value: Rc::new(Expression::New {
                    t: Rc::clone(&ty),
                    fields: None,
                }),
                ty,
            }),
        }
    }

    fn convert_lvalue_expr_in_rvalue_context(
        &self,
        lvalue: &parser::LvalueExpression,
    ) -> AnalysisResult<Typed> {
        match lvalue {
            parser::LvalueExpression::Member { lhs, member_name }
                if member_name.name == "length" =>
            {
                let Typed {
                    value: lhs,
                    ty: lhs_type,
                } = self.convert_lvalue_expr(lhs)?;

                if lhs_type.get_element_type().is_ok() {
                    // Length of array
                    Ok(Typed {
                        value: Rc::new(Expression::LengthOf {
                            arr: Rc::new(Expression::LvalueToRvalue(lhs)),
                        }),
                        ty: Rc::new(Type::Int),
                    })
                } else {
                    // Just field named `length`
                    Ok(self
                        .convert_lvalue_expr(lvalue)?
                        .map(|lvalue| Rc::new(Expression::LvalueToRvalue(lvalue))))
                }
            }
            parser::LvalueExpression::Identifier(raw_identifier) => {
                match self.lookup(raw_identifier)? {
                    Binding {
                        name,
                        decl: Decl::Var(VarDecl { t, .. }),
                    } => Ok(Typed {
                        value: Rc::new(Expression::LvalueToRvalue(Rc::new(
                            LvalueExpression::Identifier(name.clone()),
                        ))),
                        ty: Rc::clone(t),
                    }),
                    Binding {
                        decl: Decl::Const(ConstDecl { value, t }),
                        ..
                    } => Ok(Typed {
                        value: Rc::clone(value), // Constants are immediately propagated
                        ty: Rc::clone(t),
                    }),

                    t => Err(AnalysisError {
                        what: format!(
                            "{:?} is not a name of variable in lvalue expression",
                            t.name
                        ),
                    })?,
                }
            }
            parser::LvalueExpression::Member { .. } | parser::LvalueExpression::Index { .. } => {
                Ok(self
                    .convert_lvalue_expr(lvalue)?
                    .map(|lvalue| Rc::new(Expression::LvalueToRvalue(lvalue))))
            }
        }
    }

    fn convert_expr(&self, tree: &parser::Expression) -> AnalysisResult<Typed> {
        Ok(match tree {
            parser::Expression::LvalueToRvalue(lvalue_expression) => {
                self.convert_lvalue_expr_in_rvalue_context(lvalue_expression)?
            }
            parser::Expression::IntegerLiteral(integer_literal) => Typed {
                value: Rc::new(Expression::IntegerLiteral(IntegerLiteral {
                    repr: integer_literal.repr.clone(),
                    value: integer_literal.value,
                })),
                ty: Rc::new(Type::Int),
            },
            parser::Expression::RealLiteral(real_literal) => Typed {
                value: Rc::new(Expression::RealLiteral(RealLiteral {
                    repr: real_literal.repr.clone(),
                    value: real_literal.value,
                })),
                ty: Rc::new(Type::Real),
            },
            parser::Expression::BoolLiteral(bool_literal) => Typed {
                value: Rc::new(Expression::BoolLiteral(match bool_literal {
                    parser::BoolLiteral::True => BoolLiteral::True,
                    parser::BoolLiteral::False => BoolLiteral::False,
                })),
                ty: Rc::new(Type::Bool),
            },
            parser::Expression::Call { callee, args } => self.convert_call(callee, args)?,
            parser::Expression::BinOp { op, lhs, rhs } => {
                let lhs = self.convert_expr(lhs)?;
                let rhs = self.convert_expr(rhs)?;
                let BinOpAdjustment {
                    result: result_type,
                    operand: operand_type,
                    operator: semantic_op,
                } = infer_binary_operator_type(&lhs.ty, &rhs.ty, *op)?;

                let actual_lhs = cast_to(lhs, &operand_type)?;
                let actual_rhs = cast_to(rhs, &operand_type)?;

                Typed {
                    value: Rc::new(Expression::BinOp {
                        op: semantic_op,
                        lhs: actual_lhs,
                        rhs: actual_rhs,
                    }),
                    ty: result_type,
                }
            }
            parser::Expression::UnOp { op, operand } => {
                Self::convert_unary(*op, self.convert_expr(operand)?)?
            }
            parser::Expression::Cast { operand, target } => {
                let Typed {
                    value: operand,
                    ty: operand_type,
                } = self.convert_expr(operand)?;
                let target_ty = self.convert_type(target)?;
                let operand_effective_type = self.bindings.get_effective_type(&operand_type)?;
                let target_effective_type = self.bindings.get_effective_type(&target_ty)?;

                if operand_effective_type == target_effective_type {
                    Typed {
                        value: Rc::new(Expression::Cast {
                            operand,
                            target: Rc::clone(&target_ty),
                        }),
                        ty: target_ty,
                    }
                } else {
                    Err(AnalysisError {
                        what: format!("Type {operand_type:?} is incompatible with {target:?}"),
                    })?
                }
            }
            parser::Expression::New {
                t,
                fields,
                array_length,
            } => self.convert_new(t, fields.as_deref(), array_length.as_deref())?,
            parser::Expression::Null => Typed {
                value: Rc::new(Expression::Null),
                ty: Rc::new(Type::Null),
            },
        })
    }

    fn convert_for(
        &mut self,
        counter: &RawIdentifier,
        from: &parser::Expression,
        to: Option<&parser::Expression>,
        order: LoopOrder,
        body: &parser::Block,
    ) -> AnalysisResult<Statement> {
        let (result, locals_count) = self.scoped(|this| {
            let counter_loc = this.get_fresh_local_location();
            Ok(match to {
                None => {
                    let Typed {
                        value: array_expr,
                        ty: array_type,
                    } = this.convert_expr(from)?;
                    let element_type = array_type.get_element_type()?;
                    let counter_decl = LocalDecl::Var(VarDecl {
                        t: Rc::clone(element_type),
                        initialiser: None,
                        relative_location: counter_loc,
                    });

                    let counter_ident = this.bind_local_decl(counter, counter_decl);
                    let body = this.convert_block(body)?;
                    Statement::ForEach {
                        counter: counter_ident,
                        collection: array_expr,
                        order,
                        body,
                    }
                }
                Some(to) => {
                    let from = this.convert_expr(from)?;
                    let to = this.convert_expr(to)?;
                    let int_type = Rc::new(Type::Int);
                    let counter_decl = LocalDecl::Var(VarDecl {
                        t: Rc::clone(&int_type),
                        initialiser: None,
                        relative_location: counter_loc,
                    });
                    let counter_ident = this.bind_local_decl(counter, counter_decl);
                    let body = this.convert_block(body)?;
                    Statement::For {
                        counter: counter_ident,
                        lower_bound: cast_to(from, &int_type)?,
                        upper_bound: cast_to(to, &int_type)?,
                        order,
                        body,
                    }
                }
            })
        });

        assert_eq!(
            locals_count, 1,
            "Internal compiler error: mismatched number of locals in `for` counter block"
        );

        result
    }

    fn convert_stmt(&mut self, stmt: &parser::Statement) -> AnalysisResult<Statement> {
        Ok(match stmt {
            &parser::Statement::Assert { ref value, pos } => Statement::If {
                condition: Rc::new(Expression::UnOp {
                    op: UnaryOperator::BoolNeg,
                    operand: cast_to(self.convert_expr(value)?, &Type::Bool)?,
                }),
                on_true: Block {
                    stmts: vec![Statement::Panic { pos }],
                    locals_count: 0,
                },
                on_false: None,
            },
            &parser::Statement::Panic { pos } => Statement::Panic { pos },
            parser::Statement::Assignment { lhs, rhs } => {
                let Typed {
                    value: lhs,
                    ty: target_type,
                } = self.convert_lvalue_expr(lhs)?;
                Statement::Assignment {
                    lhs,
                    rhs: cast_to(self.convert_expr(rhs)?, &target_type)?,
                }
            }
            parser::Statement::While { condition, body } => Statement::While {
                condition: cast_to(self.convert_expr(condition)?, &Type::Bool)?,
                body: self.convert_block(body)?,
            },
            parser::Statement::Expr(expression) => {
                let Typed { value: expr, ty: _ } = self.convert_expr(expression)?;

                Statement::Expr(expr)
            }
            parser::Statement::If {
                condition,
                on_true,
                on_false,
            } => Statement::If {
                condition: cast_to(self.convert_expr(condition)?, &Type::Bool)?,
                on_true: self.convert_block(on_true)?,
                on_false: on_false
                    .as_ref()
                    .map(|block| self.convert_block(block))
                    .transpose()?,
            },
            parser::Statement::For {
                counter,
                from,
                to,
                order,
                body,
            } => self.convert_for(counter, from, to.as_deref(), *order, body)?,

            parser::Statement::Print { value } => {
                let Typed { value, ty: _ } = self.convert_expr(value)?;
                Statement::Print { value }
            }
            parser::Statement::Return { value } => match &self.current_routine {
                Some(RoutinePrototype { return_type, .. }) => Statement::Return {
                    value: cast_to(self.convert_expr(value)?, return_type)?,
                },
                None => Err(AnalysisError {
                    what: "Return outside of routine".to_string(),
                })?,
            },
        })
    }

    fn convert_block(&mut self, block: &parser::Block) -> AnalysisResult<Block> {
        let (result, locals_count) = self.scoped(|this| -> AnalysisResult<_> {
            block
                .0
                .iter()
                .map(|elem| {
                    Ok(match elem {
                        parser::BlockElem::Stmt(statement) => this.convert_stmt(statement)?,
                        parser::BlockElem::VarDecl(var_decl) => Statement::Declaration(
                            this.convert_var_decl(var_decl, false)?.map(LocalDecl::Var),
                        ),
                        parser::BlockElem::ConstDecl(const_decl) => Statement::Declaration(
                            this.convert_const_decl(const_decl, false)?
                                .map(LocalDecl::Const),
                        ),
                        parser::BlockElem::TypeDecl(type_decl) => Statement::Declaration(
                            this.convert_type_decl(type_decl, false)?
                                .map(LocalDecl::Type),
                        ),
                    })
                })
                .collect()
        });
        result.map(|stmts| Block {
            stmts,
            locals_count,
        })
    }

    fn convert_const_decl(
        &mut self,
        decl: &parser::ConstDecl,
        is_global: bool,
    ) -> AnalysisResult<Binding<ConstDecl>> {
        let parser::ConstDecl {
            name,
            t,
            initialiser,
        } = decl;
        let expr = self.convert_expr(initialiser)?;
        let decl = match t {
            Some(t) => {
                let t = self.convert_type(t)?;
                let expr = cast_to(expr, &t)?;
                ConstDecl {
                    t,
                    value: expr.try_constexpr_evaluate()?.as_literal(),
                }
            }
            None => ConstDecl {
                t: expr.ty,
                value: expr.value.try_constexpr_evaluate()?.as_literal(),
            },
        };

        Ok(Binding {
            decl: decl.clone(),
            name: if is_global {
                self.bind_global_decl(name, Decl::Const(decl))
            } else {
                self.bind_local_decl(name, LocalDecl::Const(decl))
            },
        })
    }

    fn convert_var_decl(
        &mut self,
        decl: &parser::VarDecl,
        is_global: bool,
    ) -> AnalysisResult<Binding<VarDecl>> {
        let parser::VarDecl {
            name,
            t,
            initialiser,
        } = decl;

        let loc = if is_global {
            self.get_fresh_global_location()
        } else {
            self.get_fresh_local_location()
        };

        let decl = match (t, initialiser) {
            (None, None) => Err(AnalysisError {
                what: format!("Can not deduce type for variable {name:?}"),
            })?,
            (None, Some(expr)) => {
                let Typed {
                    value: initialiser,
                    ty: t,
                } = self.convert_expr(expr)?;
                VarDecl {
                    t,
                    initialiser: Some(initialiser),
                    relative_location: loc,
                }
            }
            (Some(t), None) => VarDecl {
                t: self.convert_type(t)?,
                initialiser: None,
                relative_location: loc,
            },
            (Some(t), Some(expr)) => {
                let t = self.convert_type(t)?;
                VarDecl {
                    initialiser: Some(cast_to(self.convert_expr(expr)?, &t)?),
                    t,
                    relative_location: loc,
                }
            }
        };

        Ok(Binding {
            decl: decl.clone(),
            name: if is_global {
                self.bind_global_decl(name, Decl::Var(decl))
            } else {
                self.bind_local_decl(name, LocalDecl::Var(decl))
            },
        })
    }

    fn convert_type_decl(
        &mut self,
        decl: &parser::TypeDecl,
        is_global: bool,
    ) -> AnalysisResult<Binding<TypeDecl>> {
        let parser::TypeDecl { name, t } = decl;
        // Binding forward declaration of type for possible recursive usage
        let ident = {
            let forward = TypeDecl::Forward {
                alias: name.clone(),
            };
            if is_global {
                self.bind_global_decl(name, Decl::Type(forward))
            } else {
                self.bind_local_decl(name, LocalDecl::Type(forward))
            }
        };

        let prescribed = self.convert_type(t)?;
        let type_decl = TypeDecl::Full {
            effective: self.bindings.get_effective_type(&prescribed)?,
            prescribed,
        };
        // Overriding forward declaration with full one
        self.rebind_decl(&ident, Decl::Type(type_decl.clone()));
        Ok(Binding {
            name: ident,
            decl: type_decl,
        })
    }

    fn convert_global_decl(&mut self, decl: &parser::Declaration) -> AnalysisResult<Binding> {
        Ok(match decl {
            parser::Declaration::Var(decl) => self.convert_var_decl(decl, true)?.map(Decl::Var),

            parser::Declaration::Const(decl) => {
                self.convert_const_decl(decl, true)?.map(Decl::Const)
            }
            parser::Declaration::Type(decl) => self.convert_type_decl(decl, true)?.map(Decl::Type),
            parser::Declaration::Routine(decl) => self.convert_routine(decl)?,
        })
    }

    fn convert_routine(&mut self, decl: &parser::RoutineDecl) -> AnalysisResult<Binding> {
        let parser::RoutineDecl {
            name,
            arguments,
            return_type,
            body,
        } = decl;
        let return_type = return_type.as_ref();

        let argument_types = arguments
            .iter()
            .map(|(name, arg_type)| self.convert_type(arg_type).map(|t| (name.clone(), t)))
            .collect::<AnalysisResult<Vec<(RawIdentifier, Rc<Type>)>>>()?;

        let Some(body) = body else {
            let return_type = match &return_type {
                Some(t) => self.convert_type(t)?,
                None => Rc::new(Type::Unit),
            };

            let signature = RoutineSignature {
                args: argument_types,
                return_type,
            };

            let ident = self.bind_routine(
                name,
                RoutineDecl::Forward {
                    signature: signature.clone(),
                },
            )?;

            return Ok(Binding {
                name: ident,
                decl: Decl::Routine(RoutineDecl::Forward { signature }),
            });
        };

        let args_decls: Vec<_> = argument_types
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, (name, t))| {
                (
                    name,
                    VarDecl {
                        t,
                        initialiser: None,
                        relative_location: Location::Argument(
                            index.try_into().expect("Too many arguments for function"),
                        ),
                    },
                )
            })
            .collect();

        match body {
            parser::RoutineBody::Block(block) => {
                self.convert_routine_block(name, return_type, block, argument_types, &args_decls)
            }
            parser::RoutineBody::Expression(expression) => self.convert_routine_expression(
                name,
                return_type,
                expression,
                argument_types,
                &args_decls,
            ),
        }
    }

    fn convert_routine_block(
        &mut self,
        name: &RawIdentifier,
        return_type: Option<&parser::Type>,
        block: &parser::Block,
        argument_types: Vec<(RawIdentifier, Rc<Type>)>,
        args_decls: &[(RawIdentifier, VarDecl)],
    ) -> AnalysisResult<Binding> {
        let return_type = return_type.map_or(Ok(Rc::new(Type::Unit)), |t| self.convert_type(t))?;

        let signature = RoutineSignature {
            args: argument_types.clone(),
            return_type: Rc::clone(&return_type),
        };

        // Firstly, create a forward declaration for possible recursive use
        let forward_ident: Identifier = self.bind_routine(
            name,
            RoutineDecl::Forward {
                signature: signature.clone(),
            },
        )?;
        drop(forward_ident);

        // Memorise that routine for type-check of return statement
        self.current_routine = Some(RoutinePrototype {
            name: name.clone(),
            args: argument_types.into_iter().map(|(_, t)| t).collect(),
            return_type,
        });

        // create a function scope
        let ((args_bindings, body), locals_count) =
            self.scoped(|this| (this.bind_args(args_decls), this.convert_block(block)));
        assert_eq!(
            locals_count, 0,
            "Internal compiler error: locals found in function arguments block"
        );
        self.current_routine = None;

        let decl = RoutineDecl::Full(Routine {
            signature,
            args_bindings,
            body: RoutineBody::Block(body?),
        });
        Ok(Binding {
            name: self.bind_routine(name, decl.clone())?,
            decl: Decl::Routine(decl),
        })
    }

    fn bind_args(&mut self, args_decls: &[(RawIdentifier, VarDecl)]) -> Vec<Binding<VarDecl>> {
        args_decls
            .iter()
            .map(|(raw_name, decl)| {
                let decl = decl.to_owned();
                let arg_ident = self.bind_local_decl(raw_name, LocalDecl::Var(decl.clone()));
                Binding {
                    name: arg_ident,
                    decl,
                }
            })
            .collect()
    }

    fn convert_routine_expression(
        &mut self,
        name: &RawIdentifier,
        return_type: Option<&parser::Type>,
        expression: &parser::Expression,
        argument_types: Vec<(RawIdentifier, Rc<Type>)>,
        args_decls: &[(RawIdentifier, VarDecl)],
    ) -> AnalysisResult<Binding> {
        let return_type = return_type.map(|t| self.convert_type(t)).transpose()?;

        // No recursive calls for expression function

        let ((args_bindings, expr), locals_count) =
            self.scoped(|this| (this.bind_args(args_decls), this.convert_expr(expression)));
        let expr = expr?;
        assert_eq!(
            locals_count, 0,
            "Internal compiler error: locals found in function arguments block"
        );

        let (expr, return_type) = match return_type {
            Some(ty) => (cast_to(expr, &ty)?, ty),
            None => (expr.value, expr.ty),
        };

        let signature = RoutineSignature {
            args: argument_types,
            return_type,
        };

        let decl = RoutineDecl::Full(Routine {
            signature,
            args_bindings,
            body: RoutineBody::Expression(expr),
        });

        let ident = self.bind_routine(name, decl.clone())?;
        Ok(Binding {
            name: ident,
            decl: Decl::Routine(decl),
        })
    }
}

pub fn convert(program: &parser::Program) -> AnalysisResult<(Program, Bindings)> {
    let mut converter = Converter::new();

    let program = program
        .0
        .iter()
        .map(|decl| converter.convert_global_decl(decl))
        .collect::<AnalysisResult<Vec<Binding>>>()
        .map(Program)?;

    let Converter { bindings, .. } = converter;
    Ok((program, bindings))
}
