use crate::ast::{BinaryExpr, Expr, IndexAccess, UnaryExpr};
use crate::diagnostics::codes;
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext};
use crate::token::Type;

use super::super::value::ZunValue;
use super::eval_expr;

pub fn eval_index_access(
    e: &Expr,
    idx: &IndexAccess,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let obj_val = eval_expr(&idx.object, state, context, w, false)?;
    let idx_val = eval_expr(&idx.index, state, context, w, false)?;

    match obj_val {
        ZunValue::List(list) => {
            let idx = match idx_val {
                ZunValue::Int(i) => i as usize,
                ZunValue::Float(f) if f.fract() == 0.0 => f as usize,
                _ => {
                    return Err(super::runtime_error(
                        e,
                        codes::RUNTIME_INVALID_EXPRESSION,
                        "invalid list index: must be a non-negative integer",
                    ));
                }
            };
            if idx >= list.len() {
                return Err(super::runtime_error(
                    e,
                    codes::RUNTIME_INVALID_EXPRESSION,
                    format!("list index {idx} out of bounds (length: {})", list.len()),
                ));
            }
            Ok(list[idx].clone())
        }
        ZunValue::Map(map) | ZunValue::Json(map) => {
            let key = idx_val.to_string();
            match map.get(&key) {
                Some(val) => Ok(val.clone()),
                None => Err(super::runtime_error(
                    e,
                    codes::RUNTIME_INVALID_EXPRESSION,
                    format!("map key '{key}' not found"),
                )),
            }
        }
        _ => Err(super::runtime_error(
            e,
            codes::RUNTIME_INVALID_EXPRESSION,
            "index access is only supported on List, Map, and Json values",
        )),
    }
}

pub fn eval_unary(
    e: &Expr,
    u: &UnaryExpr,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
    as_identifier: bool,
) -> Result<ZunValue, Error> {
    let right = eval_expr(&u.right, state, context, w, as_identifier)?;
    match u.op {
        Type::Not => Ok(ZunValue::Bool(!right.is_truthy())),
        Type::Minus => match right {
            ZunValue::Int(n) => Ok(ZunValue::Int(-n)),
            ZunValue::Float(f) => Ok(ZunValue::Float(-f)),
            _ => Err(super::runtime_error(
                e,
                codes::RUNTIME_INVALID_EXPRESSION,
                "unary '-' only supports Int and Float values",
            )),
        },
        _ => Err(super::runtime_error(
            e,
            codes::RUNTIME_INVALID_EXPRESSION,
            "unsupported unary operator",
        )),
    }
}

pub fn eval_binary(
    e: &Expr,
    b: &BinaryExpr,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
    as_identifier: bool,
) -> Result<ZunValue, Error> {
    let left = eval_expr(&b.left, state, context, w, as_identifier)?;
    let right = eval_expr(&b.right, state, context, w, as_identifier)?;

    match b.op {
        Type::Plus => match (&left, &right) {
            (ZunValue::Int(l), ZunValue::Int(r)) => Ok(ZunValue::Int(l + r)),
            (ZunValue::Int(l), ZunValue::Float(r)) => Ok(ZunValue::Float(*l as f64 + r)),
            (ZunValue::Float(l), ZunValue::Int(r)) => Ok(ZunValue::Float(l + *r as f64)),
            (ZunValue::Float(l), ZunValue::Float(r)) => Ok(ZunValue::Float(l + r)),
            (ZunValue::Str(l), ZunValue::Str(r)) => Ok(ZunValue::Str(l.clone() + r)),
            (ZunValue::Str(l), ZunValue::Int(r)) => Ok(ZunValue::Str(format!("{l}{r}"))),
            (ZunValue::Int(l), ZunValue::Str(r)) => Ok(ZunValue::Str(format!("{l}{r}"))),
            (ZunValue::Str(l), ZunValue::Float(r)) => Ok(ZunValue::Str(format!("{l}{r}"))),
            (ZunValue::Float(l), ZunValue::Str(r)) => Ok(ZunValue::Str(format!("{l}{r}"))),
            (ZunValue::List(l), ZunValue::List(r)) => {
                let mut new_list = l.clone();
                new_list.extend(r.clone());
                Ok(ZunValue::List(new_list))
            }
            _ => Err(super::runtime_error(
                e,
                codes::RUNTIME_INVALID_EXPRESSION,
                format!("operator '+' does not support {} and {}", left, right),
            )),
        },
        Type::Minus => match (&left, &right) {
            (ZunValue::Int(l), ZunValue::Int(r)) => Ok(ZunValue::Int(l - r)),
            (ZunValue::Int(l), ZunValue::Float(r)) => Ok(ZunValue::Float(*l as f64 - r)),
            (ZunValue::Float(l), ZunValue::Int(r)) => Ok(ZunValue::Float(l - *r as f64)),
            (ZunValue::Float(l), ZunValue::Float(r)) => Ok(ZunValue::Float(l - r)),
            _ => Err(super::runtime_error(
                e,
                codes::RUNTIME_INVALID_EXPRESSION,
                format!("operator '-' does not support {} and {}", left, right),
            )),
        },
        Type::Mul => match (&left, &right) {
            (ZunValue::Int(l), ZunValue::Int(r)) => Ok(ZunValue::Int(l * r)),
            (ZunValue::Int(l), ZunValue::Float(r)) => Ok(ZunValue::Float(*l as f64 * r)),
            (ZunValue::Float(l), ZunValue::Int(r)) => Ok(ZunValue::Float(l * *r as f64)),
            (ZunValue::Float(l), ZunValue::Float(r)) => Ok(ZunValue::Float(l * r)),
            (ZunValue::Str(s), ZunValue::Int(n)) => {
                if *n >= 0 {
                    Ok(ZunValue::Str(s.repeat(*n as usize)))
                } else {
                    Err(super::runtime_error(
                        e,
                        codes::RUNTIME_INVALID_EXPRESSION,
                        "string repetition requires a non-negative integer",
                    ))
                }
            }
            (ZunValue::List(l), ZunValue::Int(n)) => {
                if *n >= 0 {
                    let mut new_list = Vec::new();
                    for _ in 0..*n as usize {
                        new_list.extend(l.clone());
                    }
                    Ok(ZunValue::List(new_list))
                } else {
                    Err(super::runtime_error(
                        e,
                        codes::RUNTIME_INVALID_EXPRESSION,
                        "list repetition requires a non-negative integer",
                    ))
                }
            }
            _ => Err(super::runtime_error(
                e,
                codes::RUNTIME_INVALID_EXPRESSION,
                format!("operator '*' does not support {} and {}", left, right),
            )),
        },
        Type::Div => numeric_division(e, &left, &right),
        Type::Mod => numeric_modulo(e, &left, &right),
        Type::Eq => Ok(ZunValue::Bool(left == right)),
        Type::Ne => Ok(ZunValue::Bool(left != right)),
        Type::Gt => compare_values(e, &left, &right, ">", |l, r| l > r, |l, r| l > r),
        Type::Lt => compare_values(e, &left, &right, "<", |l, r| l < r, |l, r| l < r),
        Type::Gte => compare_values(e, &left, &right, ">=", |l, r| l >= r, |l, r| l >= r),
        Type::Lte => compare_values(e, &left, &right, "<=", |l, r| l <= r, |l, r| l <= r),
        Type::And => Ok(ZunValue::Bool(left.is_truthy() && right.is_truthy())),
        Type::Or => Ok(ZunValue::Bool(left.is_truthy() || right.is_truthy())),
        _ => Err(super::runtime_error(
            e,
            codes::RUNTIME_INVALID_EXPRESSION,
            "unsupported binary operator",
        )),
    }
}

fn numeric_division(e: &Expr, left: &ZunValue, right: &ZunValue) -> Result<ZunValue, Error> {
    match (left, right) {
        (ZunValue::Int(l), ZunValue::Int(r)) => {
            if *r != 0 {
                if *l % *r == 0 {
                    Ok(ZunValue::Int(l / r))
                } else {
                    Ok(ZunValue::Float(*l as f64 / *r as f64))
                }
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_DIVISION_BY_ZERO,
                    "division by zero",
                ))
            }
        }
        (ZunValue::Int(l), ZunValue::Float(r)) => {
            if *r != 0.0 {
                Ok(ZunValue::Float(*l as f64 / r))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_DIVISION_BY_ZERO,
                    "division by zero",
                ))
            }
        }
        (ZunValue::Float(l), ZunValue::Int(r)) => {
            if *r != 0 {
                Ok(ZunValue::Float(l / *r as f64))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_DIVISION_BY_ZERO,
                    "division by zero",
                ))
            }
        }
        (ZunValue::Float(l), ZunValue::Float(r)) => {
            if *r != 0.0 {
                Ok(ZunValue::Float(l / r))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_DIVISION_BY_ZERO,
                    "division by zero",
                ))
            }
        }
        _ => Err(super::runtime_error(
            e,
            codes::RUNTIME_INVALID_EXPRESSION,
            format!("operator '/' does not support {} and {}", left, right),
        )),
    }
}

fn numeric_modulo(e: &Expr, left: &ZunValue, right: &ZunValue) -> Result<ZunValue, Error> {
    match (left, right) {
        (ZunValue::Int(l), ZunValue::Int(r)) => {
            if *r != 0 {
                Ok(ZunValue::Int(l % r))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_MODULO_BY_ZERO,
                    "modulo by zero",
                ))
            }
        }
        (ZunValue::Int(l), ZunValue::Float(r)) => {
            if *r != 0.0 {
                Ok(ZunValue::Float(*l as f64 % r))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_MODULO_BY_ZERO,
                    "modulo by zero",
                ))
            }
        }
        (ZunValue::Float(l), ZunValue::Int(r)) => {
            if *r != 0 {
                Ok(ZunValue::Float(l % *r as f64))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_MODULO_BY_ZERO,
                    "modulo by zero",
                ))
            }
        }
        (ZunValue::Float(l), ZunValue::Float(r)) => {
            if *r != 0.0 {
                Ok(ZunValue::Float(l % r))
            } else {
                Err(super::runtime_error(
                    e,
                    codes::RUNTIME_MODULO_BY_ZERO,
                    "modulo by zero",
                ))
            }
        }
        _ => Err(super::runtime_error(
            e,
            codes::RUNTIME_INVALID_EXPRESSION,
            format!("operator '%' does not support {} and {}", left, right),
        )),
    }
}

fn compare_values<FNum, FStr>(
    e: &Expr,
    left: &ZunValue,
    right: &ZunValue,
    op: &str,
    cmp_num: FNum,
    cmp_str: FStr,
) -> Result<ZunValue, Error>
where
    FNum: Fn(f64, f64) -> bool,
    FStr: Fn(&str, &str) -> bool,
{
    match (left, right) {
        (ZunValue::Int(l), ZunValue::Int(r)) => Ok(ZunValue::Bool(cmp_num(*l as f64, *r as f64))),
        (ZunValue::Float(l), ZunValue::Float(r)) => Ok(ZunValue::Bool(cmp_num(*l, *r))),
        (ZunValue::Int(l), ZunValue::Float(r)) => Ok(ZunValue::Bool(cmp_num(*l as f64, *r))),
        (ZunValue::Float(l), ZunValue::Int(r)) => Ok(ZunValue::Bool(cmp_num(*l, *r as f64))),
        (ZunValue::Str(l), ZunValue::Str(r)) => Ok(ZunValue::Bool(cmp_str(l, r))),
        _ => Err(super::runtime_error(
            e,
            codes::RUNTIME_INVALID_EXPRESSION,
            format!("operator '{op}' does not support {} and {}", left, right),
        )),
    }
}
