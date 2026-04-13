use core::iter;
use std::{
    collections::{HashMap, hash_map::Entry},
    rc::Rc,
};

use common::{Location, LoopOrder, RawIdentifier, VarLoc};
use indexed_arena::Arena;

use crate::{
    operators::UnaryOperator,
    tree::ConstDecl,
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

    fn add(&mut self, ident: Identifier) {
        let Identifier { raw, id } = ident;
        let _: &mut BindingId = match self.binders.entry(raw) {
            Entry::Vacant(e) => e.insert(id),
            Entry::Occupied(e) => {
                unreachable!(
                    "Duplicate bindings for the same ident ({:?}): {:?} & {id:?}",
                    e.key(),
                    e.get()
                )
            }
        };
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
    bindings: Arena<Decl, usize>,
    global_scope: Scope,
    local_scopes: Vec<Scope>,
    global_count: VarLoc,
    local_count: VarLoc,
    current_routine: Option<RoutinePrototype>,
}

struct Binding<'a> {
    id: BindingId,
    decl: &'a Decl,
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

    fn bind_decl(&mut self, name: RawIdentifier, decl: Decl, is_global: bool) -> BindingId {
        // TODO: process function forward declaration
        let id = self.bindings.alloc(decl);
        if is_global {
            &mut self.global_scope
        } else {
            self.local_scopes
                .last_mut()
                .expect("Cannot bind a local decl outside of any local scopes")
        }
        .add(Identifier { raw: name, id });
        id
    }

    fn rebind_decl(&mut self, id: BindingId, new_decl: Decl) {
        self.bindings[id] = new_decl
    }

    fn bind_routine(
        &mut self,
        routine_name: &RawIdentifier,
        decl: RoutineDecl,
    ) -> AnalysisResult<BindingId> {
        Ok(match self.lookup(routine_name) {
            Ok(Binding {
                id,
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
                    self.rebind_decl(id, Decl::Routine(decl));
                    id
                }

                [
                    RoutineDecl::Full(..) | RoutineDecl::Forward { .. },
                    RoutineDecl::Forward { .. },
                ] => id,
            },

            Ok(Binding {
                id: _,
                decl: Decl::Type(_) | Decl::Var(_) | Decl::Const(_),
            })
            | Err(_) => {
                // function just shadows previous global variable or type with the same name
                self.bind_decl(routine_name.to_owned(), Decl::Routine(decl), true)
            }
        })
    }

    fn lookup<'this>(&'this self, name: &RawIdentifier) -> AnalysisResult<Binding<'this>> {
        match self
            .local_scopes
            .iter()
            .rev()
            .chain(iter::once(&self.global_scope))
            .find_map(|scope_block| scope_block.lookup(name))
        {
            Some(id) => Ok(Binding {
                id,
                decl: &self.bindings[id],
            }),
            None => Err(AnalysisError {
                what: format!("Unknown name `{name}`"),
            }),
        }
    }

    pub(crate) fn coerce(
        &self,
        expr: Typed<Expression>,
        target_type: &Type,
    ) -> AnalysisResult<Rc<Expression>> {
        let Typed {
            value: expr,
            ty: own_type,
        } = expr;

        let source_type = own_type.as_ref();

        match [source_type, target_type] {
            [Type::Int, Type::Int]
            | [Type::Bool, Type::Bool]
            | [Type::Real, Type::Real]
            | [Type::Null, Type::Null | Type::Record(_) | Type::Array(_)] => Ok(expr),

            [Type::Null, Type::Alias(ty)] => {
                let target = Rc::clone(ty.effective());
                self.coerce(
                    Typed {
                        value: expr,
                        ty: own_type,
                    },
                    &target,
                )
            }

            [Type::Bool, Type::Real] => Ok(Rc::new(Expression::IntToReal(Rc::new(
                Expression::BoolToInt(expr),
            )))),
            [Type::Bool, Type::Int] => Ok(Rc::new(Expression::BoolToInt(expr))),
            [Type::Int, Type::Real] => Ok(Rc::new(Expression::IntToReal(expr))),

            [Type::Real, Type::Bool] => Ok(Rc::new(Expression::RealToInt(Rc::new(
                Expression::IntToBool(expr),
            )))),
            [Type::Real, Type::Int] => Ok(Rc::new(Expression::RealToInt(expr))),
            [Type::Int, Type::Bool] => Ok(Rc::new(Expression::IntToBool(expr))),

            [
                Type::Int
                | Type::Bool
                | Type::Real
                | Type::Array(_)
                | Type::Record(_)
                | Type::Alias(_),
                Type::Null,
            ] => Err(AnalysisError {
                what: format!("Cannot discard a value of type `{own_type}`"),
            }),

            [
                Type::Array(_) | Type::Record(_) | Type::Null,
                Type::Int | Type::Real | Type::Bool,
            ] => Err(AnalysisError {
                what: format!(
                    "Reference-counted type `{own_type}` cannot be converted to numeric type `{target_type}`"
                ),
            }),

            [
                Type::Int | Type::Real | Type::Bool,
                Type::Array(_) | Type::Record(_),
            ] => Err(AnalysisError {
                what: format!(
                    "Numeric type `{own_type}` cannot be converted to reference-counted type `{target_type}`"
                ),
            }),

            [Type::Alias(from), Type::Alias(to)] if from == to => Ok(expr),
            [Type::Record(r1), Type::Record(r2)] if r1 == r2 => Ok(expr),
            [
                Type::Array(ArrayDescription {
                    t: from_t,
                    length: from_length,
                }),
                Type::Array(ArrayDescription {
                    t: to_t,
                    length: to_length,
                }),
            ] if from_t == to_t && (from_length == to_length || to_length.is_none()) => Ok(expr),

            [
                Type::Array(_) | Type::Record(_),
                Type::Array(_) | Type::Record(_),
            ]
            | [Type::Alias(_), _]
            | [_, Type::Alias(_)] => Err(AnalysisError {
                what: format!(
                    "There is no implicit conversion from `{source_type}` to `{target_type}`"
                ),
            }),
        }
    }

    fn convert_type(&self, t: &parser::Type) -> AnalysisResult<Rc<Type>> {
        Ok(match t {
            parser::Type::Int => Type::int(),
            parser::Type::Real => Type::real(),
            parser::Type::Bool => Type::bool(),
            parser::Type::Alias(raw_identifier) => {
                let Binding { id: _, decl } = self.lookup(raw_identifier)?;
                Rc::new(Type::Alias(match decl {
                    Decl::Var(_) | Decl::Const(_) | Decl::Routine(_) => todo!("Report an error"),
                    Decl::Type(decl) => match decl {
                        TypeDecl::Full {
                            prescribed: _,
                            effective,
                        } => Rc::clone(effective),
                        TypeDecl::Forward { .. } => todo!("Eliminate this branch"),
                    },
                }))
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
                        id,
                        decl: Decl::Var(VarDecl { t, .. }),
                    } => Typed {
                        value: Rc::new(LvalueExpression::Binding(id)),
                        ty: Rc::clone(t),
                    },
                    _ => Err(AnalysisError {
                        what: format!(
                            "{raw_identifier:?} is not a name of variable in lvalue expression",
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
                    ty: t.effective().get_field_type(member_name)?,
                }
            }
            parser::LvalueExpression::Index { lhs, index } => {
                let Typed {
                    value: lhs,
                    ty: lhs_t,
                } = self.convert_lvalue_expr(lhs)?;
                let effective_lhs_type = lhs_t.effective();
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
                    ty: Type::bool(),
                },
                Type::Int => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::BoolNeg,
                        operand: Rc::new(Expression::IntToBool(operand)),
                    }),
                    ty: Type::bool(),
                },

                Type::Real | Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null => {
                    Err(AnalysisError {
                        what: format!(
                            "Logical negation operator can not be applied to non-boolean {operand_type:?} value"
                        ),
                    })?
                }
            },
            parser::Operator::Minus => match &*operand_type {
                Type::Int => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::IntNeg,
                        operand,
                    }),
                    ty: Type::int(),
                },
                Type::Real => Typed {
                    value: Rc::new(Expression::UnOp {
                        op: UnaryOperator::RealNeg,
                        operand,
                    }),
                    ty: Rc::new(Type::Real),
                },
                Type::Bool | Type::Alias(_) | Type::Record(_) | Type::Array(_) | Type::Null => {
                    Err(AnalysisError {
                        what: format!(
                            "Arithmetical negation operator can not be applied to non-scalar type {operand_type:?} value"
                        ),
                    })?
                }
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
        args: &[parser::Expression],
    ) -> AnalysisResult<Typed> {
        let Binding {
            id: callee_id,
            decl: callee_decl,
        } = self.lookup(callee)?;

        let RoutineSignature {
            args: formal_args,
            return_type,
        } = callee_decl.ensure_is_routine(callee)?.signature();
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
                callee: callee_id,
                args: iter::zip(arguments_types, args)
                    .map(|(arg_type, expr)| self.coerce(expr, &arg_type))
                    .collect::<AnalysisResult<_>>()?,
            }),
            ty: Rc::clone(return_type),
        })
    }

    fn convert_new(
        &self,
        t: &parser::Type,
        fields: Option<&[(RawIdentifier, parser::Expression)]>,
        array_length: Option<&parser::Expression>,
    ) -> AnalysisResult<Typed> {
        let ty = self.convert_type(t)?;
        let effective = ty.effective();

        if let Some(length) = array_length {
            let Type::Array(ArrayDescription {
                t: elements,
                length: None,
            }) = effective.as_ref()
            else {
                return Err(AnalysisError {
                    what: "new[] is only supported for array[] types without length".to_string(),
                });
            };
            return Ok(Typed {
                value: Rc::new(Expression::NewArray {
                    elements: Rc::clone(elements),
                    length: self.coerce(self.convert_expr(length)?, &Type::Int)?,
                }),
                ty,
            });
        }

        match effective.as_ref() {
            Type::Int | Type::Real | Type::Bool | Type::Null => Err(AnalysisError {
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
                                        self.coerce(expr, record.get_field_type(name)?.as_ref())?,
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
                        ty: Type::int(),
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
                        id,
                        decl: Decl::Var(VarDecl { t, .. }),
                    } => Ok(Typed {
                        value: Rc::new(Expression::LvalueToRvalue(Rc::new(
                            LvalueExpression::Binding(id),
                        ))),
                        ty: Rc::clone(t),
                    }),
                    Binding {
                        decl: Decl::Const(ConstDecl { value }),
                        ..
                    } => Ok(value.as_literal().into()), // Constants are immediately propagated

                    _ => Err(AnalysisError {
                        what: format!(
                            "{raw_identifier:?} is not a name of variable in lvalue expression",
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
            parser::Expression::Literal(literal) => literal.clone().into(),
            parser::Expression::Call { callee, args } => self.convert_call(callee, args)?,
            parser::Expression::BinOp { op, lhs, rhs } => {
                let lhs = self.convert_expr(lhs)?;
                let rhs = self.convert_expr(rhs)?;
                let BinOpAdjustment {
                    result: result_type,
                    operand: operand_type,
                    operator: semantic_op,
                } = infer_binary_operator_type(&lhs.ty, &rhs.ty, *op)?;

                let actual_lhs = self.coerce(lhs, &operand_type)?;
                let actual_rhs = self.coerce(rhs, &operand_type)?;

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
                let operand_effective_type = operand_type.effective();
                let target_effective_type = target_ty.effective();

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
                ty: Type::null(),
            },
        })
    }

    fn convert_for(
        &mut self,
        counter: RawIdentifier,
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
                    let counter_decl = Decl::Var(VarDecl {
                        t: Rc::clone(element_type),
                        initialiser: element_type.get_default_initialiser().into(),
                        relative_location: counter_loc,
                    });

                    let counter = this.bind_decl(counter, counter_decl, false);
                    let body = this.convert_block(body)?;
                    Statement::ForEach {
                        counter,
                        collection: array_expr,
                        order,
                        body,
                    }
                }
                Some(to) => {
                    let from = this.convert_expr(from)?;
                    let to = this.convert_expr(to)?;
                    let int_type = Type::int();
                    let counter_decl = Decl::Var(VarDecl {
                        t: Rc::clone(&int_type),
                        initialiser: int_type.get_default_initialiser().into(),
                        relative_location: counter_loc,
                    });
                    let counter_ident = this.bind_decl(counter, counter_decl, false);
                    let body = this.convert_block(body)?;
                    Statement::For {
                        counter: counter_ident,
                        lower_bound: this.coerce(from, &int_type)?,
                        upper_bound: this.coerce(to, &int_type)?,
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
                    operand: self.coerce(self.convert_expr(value)?, &Type::Bool)?,
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
                    rhs: self.coerce(self.convert_expr(rhs)?, &target_type)?,
                }
            }
            parser::Statement::While { condition, body } => Statement::While {
                condition: self.coerce(self.convert_expr(condition)?, &Type::Bool)?,
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
                condition: self.coerce(self.convert_expr(condition)?, &Type::Bool)?,
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
            } => self.convert_for(counter.to_owned(), from, to.as_ref(), *order, body)?,

            parser::Statement::Print { value } => {
                let Typed { value, ty: _ } = self.convert_expr(value)?;
                Statement::Print { value }
            }
            parser::Statement::Return { value } => match &self.current_routine {
                Some(RoutinePrototype { return_type, .. }) => Statement::Return {
                    value: self.coerce(self.convert_expr(value)?, return_type)?,
                },
                None => Err(AnalysisError {
                    what: "Return outside of routine".to_string(),
                })?,
            },
        })
    }

    fn convert_block(&mut self, block: &parser::Block) -> AnalysisResult<Block> {
        let (result, locals_count) = self.scoped(|this| -> AnalysisResult<_> {
            let mut result = vec![];
            for elem in &block.0 {
                result.push(match elem {
                    parser::BlockElem::Stmt(statement) => this.convert_stmt(statement)?,
                    parser::BlockElem::VarDecl(var_decl) => {
                        let binding = this.convert_var_decl(var_decl, false)?;
                        todo!("Do something abot {binding:?}")
                    }
                    parser::BlockElem::ConstDecl(const_decl) => {
                        let _: BindingId = this.convert_const_decl(const_decl, false)?;
                        continue;
                    }
                    parser::BlockElem::TypeDecl(type_decl) => {
                        let _: BindingId = this.convert_type_decl(type_decl, false)?;
                        continue;
                    }
                })
            }
            Ok(result)
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
    ) -> AnalysisResult<BindingId> {
        let parser::ConstDecl {
            name,
            t,
            initialiser,
        } = decl;
        let expr = self.convert_expr(initialiser)?;
        let decl = match t {
            Some(t) => {
                let t = self.convert_type(t)?;
                let expr = self.coerce(expr, &t)?;
                ConstDecl {
                    value: expr.try_constexpr_evaluate()?,
                }
            }
            None => ConstDecl {
                value: expr.value.try_constexpr_evaluate()?,
            },
        };
        Ok(self.bind_decl(name.to_owned(), Decl::Const(decl), is_global))
    }

    fn convert_var_decl(
        &mut self,
        decl: &parser::VarDecl,
        is_global: bool,
    ) -> AnalysisResult<BindingId> {
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
                    initialiser,
                    relative_location: loc,
                }
            }
            (Some(t), None) => {
                let t = self.convert_type(t)?;
                VarDecl {
                    initialiser: t.get_default_initialiser().into(),
                    t,
                    relative_location: loc,
                }
            }
            (Some(t), Some(expr)) => {
                let t = self.convert_type(t)?;
                VarDecl {
                    initialiser: self.coerce(self.convert_expr(expr)?, &t)?,
                    t,
                    relative_location: loc,
                }
            }
        };

        Ok(self.bind_decl(name.to_owned(), Decl::Var(decl), is_global))
    }

    fn convert_type_decl(
        &mut self,
        decl: &parser::TypeDecl,
        is_global: bool,
    ) -> AnalysisResult<BindingId> {
        let parser::TypeDecl { name, t } = decl;
        // Binding forward declaration of type for possible recursive usage
        let ident = self.bind_decl(
            name.to_owned(),
            Decl::Type(TypeDecl::Forward {
                alias: name.clone(),
            }),
            is_global,
        );

        let prescribed = self.convert_type(t)?;
        let type_decl = TypeDecl::Full {
            effective: Rc::clone(prescribed.effective()),
            prescribed,
        };
        // Overriding forward declaration with full one
        self.rebind_decl(ident, Decl::Type(type_decl));
        Ok(ident)
    }

    fn convert_global_decl(&mut self, decl: &parser::Declaration) -> AnalysisResult<BindingId> {
        Ok(match decl {
            parser::Declaration::Var(decl) => self.convert_var_decl(decl, true)?,

            parser::Declaration::Const(decl) => self.convert_const_decl(decl, true)?,
            parser::Declaration::Type(decl) => self.convert_type_decl(decl, true)?,
            parser::Declaration::Routine(decl) => self.convert_routine(decl)?,
        })
    }

    fn convert_routine(&mut self, decl: &parser::RoutineDecl) -> AnalysisResult<BindingId> {
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
                None => Type::null(),
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

            return Ok(ident);
        };

        let args_decls: Vec<_> = argument_types
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, (name, t))| {
                (
                    name,
                    VarDecl {
                        initialiser: t.get_default_initialiser().into(),
                        t,
                        relative_location: Location::Argument(
                            index.try_into().expect("Too many arguments for function"),
                        ),
                    },
                )
            })
            .collect();

        match body {
            parser::RoutineBody::Block(block) => {
                self.convert_routine_block(name, return_type, block, argument_types, args_decls)
            }
            parser::RoutineBody::Expression(expression) => self.convert_routine_expression(
                name,
                return_type,
                expression,
                argument_types,
                args_decls,
            ),
        }
    }

    fn convert_routine_block(
        &mut self,
        name: &RawIdentifier,
        return_type: Option<&parser::Type>,
        block: &parser::Block,
        argument_types: Vec<(RawIdentifier, Rc<Type>)>,
        args_decls: Vec<(RawIdentifier, VarDecl)>,
    ) -> AnalysisResult<BindingId> {
        let return_type = return_type.map_or(Ok(Type::null()), |t| self.convert_type(t))?;

        let signature = RoutineSignature {
            args: argument_types.clone(),
            return_type: Rc::clone(&return_type),
        };

        // Firstly, create a forward declaration for possible recursive use
        let _: BindingId = self.bind_routine(
            name,
            RoutineDecl::Forward {
                signature: signature.clone(),
            },
        )?;

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

        self.bind_routine(
            name,
            RoutineDecl::Full(Routine {
                signature,
                args_bindings,
                body: RoutineBody::Block(body?),
            }),
        )
    }

    fn bind_args(&mut self, args_decls: Vec<(RawIdentifier, VarDecl)>) -> Vec<BindingId> {
        args_decls
            .into_iter()
            .map(|(raw_name, decl)| self.bind_decl(raw_name, Decl::Var(decl), false))
            .collect()
    }

    fn convert_routine_expression(
        &mut self,
        name: &RawIdentifier,
        return_type: Option<&parser::Type>,
        expression: &parser::Expression,
        argument_types: Vec<(RawIdentifier, Rc<Type>)>,
        args_decls: Vec<(RawIdentifier, VarDecl)>,
    ) -> AnalysisResult<BindingId> {
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
            Some(ty) => (self.coerce(expr, &ty)?, ty),
            None => (expr.value, expr.ty),
        };

        let signature = RoutineSignature {
            args: argument_types,
            return_type,
        };

        self.bind_routine(
            name,
            RoutineDecl::Full(Routine {
                signature,
                args_bindings,
                body: RoutineBody::Expression(expr),
            }),
        )
    }
}

pub fn convert(program: &parser::Program) -> AnalysisResult<Program> {
    let mut converter = Converter::new();

    let program: Vec<_> = program
        .0
        .iter()
        .map(|decl| converter.convert_global_decl(decl))
        .collect::<AnalysisResult<_>>()?;

    let Converter { bindings, .. } = converter;
    Ok(Program {
        bindings,
        globals: program,
    })
}
