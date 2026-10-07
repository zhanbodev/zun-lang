use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "now".into(),
        Arc::new(|_args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(Utc::now().timestamp()))
        }),
    );
    exports.insert(
        "now_ms".into(),
        Arc::new(|_args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(Utc::now().timestamp_millis()))
        }),
    );
    exports.insert(
        "format".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let ts = int_arg("time.format", args, 0)?;
            let fmt = string_arg("time.format", args, 1)?;
            Ok(ZunValue::Str(seconds_to_utc(ts)?.format(fmt).to_string()))
        }),
    );
    exports.insert(
        "parse".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let input = string_arg("time.parse", args, 0)?;
            let fmt = string_arg("time.parse", args, 1)?;
            Ok(ZunValue::Int(timestamp_from_format(input, fmt)?))
        }),
    );
    exports.insert(
        "year".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                seconds_to_utc(int_arg("time.year", args, 0)?)?.year() as i64,
            ))
        }),
    );
    exports.insert(
        "month".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                seconds_to_utc(int_arg("time.month", args, 0)?)?.month() as i64,
            ))
        }),
    );
    exports.insert(
        "day".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                seconds_to_utc(int_arg("time.day", args, 0)?)?.day() as i64,
            ))
        }),
    );
    exports.insert(
        "hour".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                seconds_to_utc(int_arg("time.hour", args, 0)?)?.hour() as i64,
            ))
        }),
    );
    exports.insert(
        "minute".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                seconds_to_utc(int_arg("time.minute", args, 0)?)?.minute() as i64,
            ))
        }),
    );
    exports.insert(
        "second".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                seconds_to_utc(int_arg("time.second", args, 0)?)?.second() as i64,
            ))
        }),
    );
    exports.insert(
        "weekday".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(
                seconds_to_utc(int_arg("time.weekday", args, 0)?)?
                    .format("%A")
                    .to_string(),
            ))
        }),
    );
    exports.insert(
        "add_days".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let ts = int_arg("time.add_days", args, 0)?;
            let days = int_arg("time.add_days", args, 1)?;
            let shifted = seconds_to_utc(ts)?
                .checked_add_signed(Duration::days(days))
                .ok_or_else(|| {
                    Error::Interpreter(format!(
                        "time.add_days: timestamp overflow for ts={ts}, days={days}"
                    ))
                })?;
            Ok(ZunValue::Int(shifted.timestamp()))
        }),
    );
    exports.insert(
        "diff_days".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let lhs = int_arg("time.diff_days", args, 0)?;
            let rhs = int_arg("time.diff_days", args, 1)?;
            Ok(ZunValue::Int((lhs - rhs) / 86_400))
        }),
    );

    context.register_native_module("std.time", exports);
}

fn int_arg(id: &str, args: &[ZunValue], index: usize) -> Result<i64, Error> {
    match args.get(index) {
        Some(ZunValue::Int(n)) => Ok(*n),
        Some(other) => Err(Error::Interpreter(format!(
            "{id}: argument {} must be Int, got {}",
            index + 1,
            other.type_name()
        ))),
        None => Err(Error::Interpreter(format!(
            "{id}: missing argument {}",
            index + 1
        ))),
    }
}

fn string_arg<'a>(id: &str, args: &'a [ZunValue], index: usize) -> Result<&'a str, Error> {
    match args.get(index) {
        Some(ZunValue::Str(s)) => Ok(s),
        Some(other) => Err(Error::Interpreter(format!(
            "{id}: argument {} must be String, got {}",
            index + 1,
            other.type_name()
        ))),
        None => Err(Error::Interpreter(format!(
            "{id}: missing argument {}",
            index + 1
        ))),
    }
}

fn seconds_to_utc(ts: i64) -> Result<DateTime<Utc>, Error> {
    DateTime::from_timestamp(ts, 0)
        .ok_or_else(|| Error::Interpreter(format!("time: invalid timestamp {ts}")))
}

fn timestamp_from_format(input: &str, fmt: &str) -> Result<i64, Error> {
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(input, fmt) {
        return Ok(Utc.from_utc_datetime(&naive_dt).timestamp());
    }

    if let Ok(naive_date) = NaiveDate::parse_from_str(input, fmt) {
        let midnight = naive_date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| Error::Interpreter("time.parse: invalid midnight value".into()))?;
        return Ok(Utc.from_utc_datetime(&midnight).timestamp());
    }

    Err(Error::Interpreter(format!(
        "time.parse: unable to parse '{}' with format '{}'",
        input, fmt
    )))
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::{seconds_to_utc, timestamp_from_format};

    #[test]
    fn parses_epoch_date_as_zero() {
        assert_eq!(timestamp_from_format("1970-01-01", "%Y-%m-%d").unwrap(), 0);
    }

    #[test]
    fn exposes_epoch_components_in_utc() {
        let dt = seconds_to_utc(0).unwrap();
        assert_eq!(dt.year(), 1970);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }
}
