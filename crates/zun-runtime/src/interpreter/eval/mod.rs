mod calls;
mod constructors;
mod io;
mod literals;
mod operators;

use crate::ast::Expr;
use crate::diagnostics::{Diagnostic, codes};
use crate::error::Error;
use crate::runtime::{ProgramState, RuntimeContext};

use super::value::ZunValue;

pub fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() { "inf" } else { "-inf" }.to_string();
    }

    let abs_n = n.abs();
    let digit_count = if abs_n > 0.0 {
        abs_n.log10().floor() as i32 + 1
    } else {
        0
    };
    if digit_count > 12 || (abs_n > 0.0 && abs_n < 1e-7) {
        return format_scientific(n, 10);
    }

    if abs_n.fract().abs() < 1e-10 || abs_n >= 1e15 {
        return format!("{}", n.round() as i64);
    }

    format_float(n, 10)
}

fn format_float(n: f64, max_sig_digits: usize) -> String {
    let s = format!("{:.*}", max_sig_digits, n);

    if let Some(pos) = s.find('.') {
        let mut result = s.clone();
        while result.ends_with('0') && result.len() > pos + 1 {
            result.pop();
        }
        if result.ends_with('.') {
            result.pop();
        }
        return result;
    }

    s
}

fn format_scientific(n: f64, max_sig_digits: usize) -> String {
    if n == 0.0 {
        return "0".to_string();
    }

    let abs_n = n.abs();
    let exp = (abs_n.log10().floor()) as i32;
    let mantissa = n / 10f64.powi(exp);
    let decimals = max_sig_digits - 1;
    let factor = 10f64.powi(decimals as i32);
    let rounded = (mantissa * factor).round() / factor;

    if rounded.abs() >= 10.0 {
        let mantissa = rounded / 10.0;
        return format!("{:.prec$}e{}", mantissa, exp + 1, prec = decimals - 1);
    }

    format!("{:.prec$}e{}", rounded, exp, prec = decimals)
}

pub fn check_eval_result(val: Result<ZunValue, Error>) -> Result<ZunValue, Error> {
    val
}

fn runtime_error(expr: &Expr, code: &'static str, message: impl Into<String>) -> Error {
    Error::Diagnostic(Diagnostic::error(code, message).with_span(expr.span()))
}

fn undefined_variable_error(expr: &Expr, name: &str) -> Error {
    runtime_error(
        expr,
        codes::RUNTIME_UNDEFINED_VARIABLE,
        format!("variable '{name}' is not defined"),
    )
}

pub fn eval_expr(
    e: &Expr,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
    as_identifier: bool,
) -> Result<ZunValue, Error> {
    match e {
        Expr::Number(n) => literals::eval_number(n),
        Expr::Char(c) => literals::eval_char(c),
        Expr::Bool(b) => literals::eval_bool(b),
        Expr::Null(_) => Ok(ZunValue::Null),
        Expr::StringLiteral(s) => literals::eval_string_literal(s),
        Expr::FString(fs) => literals::eval_fstring(fs, state, context, w),
        Expr::ListLiteral(list) => literals::eval_list_literal(list, state, context, w),
        Expr::MapLiteral(map) => literals::eval_map_literal(map, state, context, w),
        Expr::VarLookup(v) => literals::eval_var_lookup(e, v, state, as_identifier),
        Expr::FnLiteral(lit) => literals::eval_fn_literal(lit, state),
        Expr::IndexAccess(idx) => operators::eval_index_access(e, idx, state, context, w),
        Expr::Unary(u) => operators::eval_unary(e, u, state, context, w, as_identifier),
        Expr::Binary(b) => operators::eval_binary(e, b, state, context, w, as_identifier),
        Expr::MethodCall(call) => calls::eval_method_call(e, call, state, context, w),
        Expr::FnCall(call) => calls::eval_fn_call(e, call, state, context, w),
        Expr::Read(read_expr) => io::eval_read_expr(e, read_expr, state, context, w),
        Expr::ConfigRead(config) => io::eval_config_read_expr(e, config, context),
        Expr::HdrRead(hdr) => io::eval_hdr_read_expr(hdr, state),
        Expr::JsonConstructor(json) => constructors::eval_json_constructor(json, state, context, w),
        Expr::HtmlConstructor(html) => constructors::eval_html_constructor(html, state, context, w),
        Expr::ResConstructor(res) => constructors::eval_res_constructor(e, res, state, context, w),
        Expr::TypeInstance(ctor) => constructors::eval_struct_constructor(ctor, state, context, w),
    }
}
