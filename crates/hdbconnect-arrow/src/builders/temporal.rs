//! Temporal type builders for date and time Arrow arrays.
//!
//! Implements builders for:
//! - Date32 (HANA DAYDATE)
//! - Time64(Nanosecond) (HANA SECONDTIME)
//! - Timestamp(Nanosecond, None) (HANA LONGDATE, SECONDDATE)
//!
//! # Conversion Approach
//!
//! HANA temporal types expose their values via Display trait as ISO strings.
//! We parse these strings to extract the numeric values for Arrow.

use std::sync::Arc;

use arrow_array::ArrayRef;
use arrow_array::builder::{Date32Builder, Time64NanosecondBuilder, TimestampNanosecondBuilder};

use crate::Result;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::sealed::private::Sealed;

// ═══════════════════════════════════════════════════════════════════════════
// Date32 Builder (Days since Unix epoch)
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for Arrow Date32 arrays (HANA DAYDATE).
///
/// Date32 represents days since Unix epoch (1970-01-01).
#[derive(Debug)]
pub struct Date32BuilderWrapper {
    builder: Date32Builder,
    len: usize,
}

impl Date32BuilderWrapper {
    /// Create a new date builder.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            builder: Date32Builder::with_capacity(capacity),
            len: 0,
        }
    }

    /// Parse YYYY-MM-DD string and convert to days since Unix epoch.
    fn parse_date_string(s: &str) -> Result<i32> {
        // Format: "YYYY-MM-DD"
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(crate::ArrowConversionError::value_conversion(
                "date",
                format!("invalid date format: {s}"),
            ));
        }

        let year: i32 = parts[0].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion("date", format!("invalid year in: {s}"))
        })?;
        let month: u32 = parts[1].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion("date", format!("invalid month in: {s}"))
        })?;
        let day: u32 = parts[2].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion("date", format!("invalid day in: {s}"))
        })?;

        // Calculate days since Unix epoch (1970-01-01)
        // Using a simplified algorithm
        Ok(days_from_ymd(year, month, day))
    }

    /// Convert HANA date value to days since epoch.
    fn hana_date_to_days(value: &hdbconnect::HdbValue) -> Result<i32> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::DAYDATE(dd) => {
                // DayDate implements Display as "YYYY-MM-DD"
                let s = dd.to_string();
                Self::parse_date_string(&s)
            }
            other => Err(crate::ArrowConversionError::value_conversion(
                "date",
                format!("expected date type, got {other:?}"),
            )),
        }
    }
}

impl Sealed for Date32BuilderWrapper {}

impl HanaCompatibleBuilder for Date32BuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        let days = Self::hana_date_to_days(value)?;
        self.builder.append_value(days);
        self.len += 1;
        Ok(())
    }

    fn append_null(&mut self) {
        self.builder.append_null();
        self.len += 1;
    }

    fn finish(&mut self) -> ArrayRef {
        self.len = 0;
        Arc::new(self.builder.finish())
    }

    fn len(&self) -> usize {
        self.len
    }

    fn capacity(&self) -> Option<usize> {
        Some(self.builder.capacity())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Time64 Builder (Nanoseconds since midnight)
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for Arrow Time64(Nanosecond) arrays (HANA SECONDTIME).
///
/// Time64 represents nanoseconds since midnight.
#[derive(Debug)]
pub struct Time64NanosecondBuilderWrapper {
    builder: Time64NanosecondBuilder,
    len: usize,
}

impl Time64NanosecondBuilderWrapper {
    /// Create a new time builder.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            builder: Time64NanosecondBuilder::with_capacity(capacity),
            len: 0,
        }
    }

    /// Parse HH:MM:SS string and convert to nanoseconds since midnight.
    fn parse_time_string(s: &str) -> Result<i64> {
        // Format: "HH:MM:SS"
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err(crate::ArrowConversionError::value_conversion(
                "time",
                format!("invalid time format: {s}"),
            ));
        }

        let hour: u32 = parts[0].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion("time", format!("invalid hour in: {s}"))
        })?;
        let minute: u32 = parts[1].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion("time", format!("invalid minute in: {s}"))
        })?;
        let second: u32 = parts[2].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion("time", format!("invalid second in: {s}"))
        })?;

        // Convert to nanoseconds since midnight
        let total_seconds = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
        Ok(total_seconds * 1_000_000_000)
    }

    /// Convert HANA time to nanoseconds since midnight.
    fn hana_time_to_nanos(value: &hdbconnect::HdbValue) -> Result<i64> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::SECONDTIME(st) => {
                // SecondTime implements Display as "HH:MM:SS"
                let s = st.to_string();
                Self::parse_time_string(&s)
            }
            other => Err(crate::ArrowConversionError::value_conversion(
                "time",
                format!("expected time type, got {other:?}"),
            )),
        }
    }
}

impl Sealed for Time64NanosecondBuilderWrapper {}

impl HanaCompatibleBuilder for Time64NanosecondBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        let nanos = Self::hana_time_to_nanos(value)?;
        self.builder.append_value(nanos);
        self.len += 1;
        Ok(())
    }

    fn append_null(&mut self) {
        self.builder.append_null();
        self.len += 1;
    }

    fn finish(&mut self) -> ArrayRef {
        self.len = 0;
        Arc::new(self.builder.finish())
    }

    fn len(&self) -> usize {
        self.len
    }

    fn capacity(&self) -> Option<usize> {
        Some(self.builder.capacity())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Timestamp Builder (Nanoseconds since Unix epoch)
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for Arrow Timestamp(Nanosecond, None) arrays (HANA LONGDATE, SECONDDATE).
///
/// Timestamp represents nanoseconds since Unix epoch (1970-01-01 00:00:00 UTC).
#[derive(Debug)]
pub struct TimestampNanosecondBuilderWrapper {
    builder: TimestampNanosecondBuilder,
    len: usize,
}

impl TimestampNanosecondBuilderWrapper {
    /// Create a new timestamp builder.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            builder: TimestampNanosecondBuilder::with_capacity(capacity),
            len: 0,
        }
    }

    /// Parse ISO datetime string and convert to nanoseconds since epoch.
    ///
    /// Formats:
    /// - `LongDate`: "YYYY-MM-DDTHH:MM:SS.FFFFFFF" (7 decimal places = 100ns)
    /// - `SecondDate`: "YYYY-MM-DDTHH:MM:SS"
    fn parse_datetime_string(s: &str) -> Result<i64> {
        // Split date and time parts
        let parts: Vec<&str> = s.split('T').collect();
        if parts.len() != 2 {
            return Err(crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid datetime format: {s}"),
            ));
        }

        // Parse date part
        let date_parts: Vec<&str> = parts[0].split('-').collect();
        if date_parts.len() != 3 {
            return Err(crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid date in: {s}"),
            ));
        }

        let year: i32 = date_parts[0].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid year in: {s}"),
            )
        })?;
        let month: u32 = date_parts[1].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid month in: {s}"),
            )
        })?;
        let day: u32 = date_parts[2].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid day in: {s}"),
            )
        })?;

        // Parse time part (may include fractional seconds)
        let time_str = parts[1];
        let (time_part, frac_nanos) = time_str.find('.').map_or((time_str, 0), |dot_pos| {
            let frac_str = &time_str[dot_pos + 1..];
            // HANA LongDate uses 7 decimal places (100-nanosecond precision)
            // Pad or truncate to 9 digits for nanoseconds
            let padded = format!("{frac_str:0<9}");
            let frac: i64 = padded[..9].parse().unwrap_or(0);
            (&time_str[..dot_pos], frac)
        });

        let time_parts: Vec<&str> = time_part.split(':').collect();
        if time_parts.len() != 3 {
            return Err(crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid time in: {s}"),
            ));
        }

        let hour: u32 = time_parts[0].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid hour in: {s}"),
            )
        })?;
        let minute: u32 = time_parts[1].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid minute in: {s}"),
            )
        })?;
        let second: u32 = time_parts[2].parse().map_err(|_| {
            crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("invalid second in: {s}"),
            )
        })?;

        // Calculate total nanoseconds since epoch
        let days = days_from_ymd(year, month, day);
        let day_nanos = i64::from(days) * 86_400 * 1_000_000_000;
        let time_nanos =
            (i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second)) * 1_000_000_000;

        Ok(day_nanos + time_nanos + frac_nanos)
    }

    /// Convert HANA timestamp to nanoseconds since epoch.
    fn hana_timestamp_to_nanos(value: &hdbconnect::HdbValue) -> Result<i64> {
        use hdbconnect::HdbValue;

        match value {
            HdbValue::LONGDATE(ld) => {
                // LongDate: "YYYY-MM-DDTHH:MM:SS.FFFFFFF"
                let s = ld.to_string();
                Self::parse_datetime_string(&s)
            }
            HdbValue::SECONDDATE(sd) => {
                // SecondDate: "YYYY-MM-DDTHH:MM:SS"
                let s = sd.to_string();
                Self::parse_datetime_string(&s)
            }
            other => Err(crate::ArrowConversionError::value_conversion(
                "timestamp",
                format!("expected timestamp type, got {other:?}"),
            )),
        }
    }
}

impl Sealed for TimestampNanosecondBuilderWrapper {}

impl HanaCompatibleBuilder for TimestampNanosecondBuilderWrapper {
    fn append_hana_value(&mut self, value: &hdbconnect::HdbValue) -> Result<()> {
        let nanos = Self::hana_timestamp_to_nanos(value)?;
        self.builder.append_value(nanos);
        self.len += 1;
        Ok(())
    }

    fn append_null(&mut self) {
        self.builder.append_null();
        self.len += 1;
    }

    fn finish(&mut self) -> ArrayRef {
        self.len = 0;
        Arc::new(self.builder.finish())
    }

    fn len(&self) -> usize {
        self.len
    }

    fn capacity(&self) -> Option<usize> {
        Some(self.builder.capacity())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Calculate days since Unix epoch (1970-01-01) from year, month, day.
///
/// Uses a simplified algorithm that handles leap years correctly.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
const fn days_from_ymd(year: i32, month: u32, day: u32) -> i32 {
    // Algorithm from https://howardhinnant.github.io/date_algorithms.html
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i32 - 719_468
}

#[cfg(test)]
mod tests {
    use arrow_array::{Array, Date32Array, Time64NanosecondArray, TimestampNanosecondArray};

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // Date32 Builder Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_date32_builder_creation() {
        let builder = Date32BuilderWrapper::new(100);
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_date32_builder_capacity() {
        let builder = Date32BuilderWrapper::new(100);
        assert!(builder.capacity().is_some());
    }

    #[test]
    fn test_date32_builder_append_null() {
        let mut builder = Date32BuilderWrapper::new(10);
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let date_array = array.as_any().downcast_ref::<Date32Array>().unwrap();
        assert!(date_array.is_null(0));
    }

    #[test]
    fn test_date32_builder_finish_resets_len() {
        let mut builder = Date32BuilderWrapper::new(10);
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let _array = builder.finish();
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Time64 Builder Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_time64_builder_creation() {
        let builder = Time64NanosecondBuilderWrapper::new(100);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_time64_builder_capacity() {
        let builder = Time64NanosecondBuilderWrapper::new(100);
        assert!(builder.capacity().is_some());
    }

    #[test]
    fn test_time64_builder_append_null() {
        let mut builder = Time64NanosecondBuilderWrapper::new(10);
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let time_array = array.as_any().downcast_ref::<Time64NanosecondArray>().unwrap();
        assert!(time_array.is_null(0));
    }

    #[test]
    fn test_time64_builder_finish_resets_len() {
        let mut builder = Time64NanosecondBuilderWrapper::new(10);
        builder.append_null();
        let _array = builder.finish();
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Timestamp Builder Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_timestamp_builder_creation() {
        let builder = TimestampNanosecondBuilderWrapper::new(100);
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn test_timestamp_builder_capacity() {
        let builder = TimestampNanosecondBuilderWrapper::new(100);
        assert!(builder.capacity().is_some());
    }

    #[test]
    fn test_timestamp_builder_append_null() {
        let mut builder = TimestampNanosecondBuilderWrapper::new(10);
        builder.append_null();
        assert_eq!(builder.len(), 1);

        let array = builder.finish();
        let ts_array = array.as_any().downcast_ref::<TimestampNanosecondArray>().unwrap();
        assert!(ts_array.is_null(0));
    }

    #[test]
    fn test_timestamp_builder_finish_resets_len() {
        let mut builder = TimestampNanosecondBuilderWrapper::new(10);
        builder.append_null();
        let _array = builder.finish();
        assert_eq!(builder.len(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // days_from_ymd Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_days_from_ymd_epoch() {
        assert_eq!(days_from_ymd(1970, 1, 1), 0);
    }

    #[test]
    fn test_days_from_ymd_day_after_epoch() {
        assert_eq!(days_from_ymd(1970, 1, 2), 1);
    }

    #[test]
    fn test_days_from_ymd_before_epoch() {
        assert_eq!(days_from_ymd(1969, 12, 31), -1);
    }

    #[test]
    fn test_days_from_ymd_year_2000() {
        assert_eq!(days_from_ymd(2000, 1, 1), 10957);
    }

    #[test]
    fn test_days_from_ymd_leap_year() {
        // 2000 is a leap year
        assert_eq!(days_from_ymd(2000, 2, 29), 10957 + 31 + 28);
        // 2024 is a leap year
        let days_2024_feb_29 = days_from_ymd(2024, 2, 29);
        let days_2024_mar_1 = days_from_ymd(2024, 3, 1);
        assert_eq!(days_2024_mar_1 - days_2024_feb_29, 1);
    }

    #[test]
    fn test_days_from_ymd_end_of_year() {
        assert_eq!(days_from_ymd(1970, 12, 31), 364);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // parse_date_string Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_date_string_epoch() {
        assert_eq!(Date32BuilderWrapper::parse_date_string("1970-01-01").unwrap(), 0);
    }

    #[test]
    fn test_parse_date_string_future() {
        assert_eq!(Date32BuilderWrapper::parse_date_string("2024-06-15").unwrap(), 19889);
    }

    #[test]
    fn test_parse_date_string_invalid_format() {
        let result = Date32BuilderWrapper::parse_date_string("2024/06/15");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_value_conversion());
    }

    #[test]
    fn test_parse_date_string_invalid_year() {
        let result = Date32BuilderWrapper::parse_date_string("XXXX-01-01");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_string_invalid_month() {
        let result = Date32BuilderWrapper::parse_date_string("2024-XX-01");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_date_string_invalid_day() {
        let result = Date32BuilderWrapper::parse_date_string("2024-06-XX");
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // parse_time_string Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_time_string_midnight() {
        assert_eq!(Time64NanosecondBuilderWrapper::parse_time_string("00:00:00").unwrap(), 0);
    }

    #[test]
    fn test_parse_time_string_midday() {
        let expected = 12 * 3600 * 1_000_000_000_i64;
        assert_eq!(Time64NanosecondBuilderWrapper::parse_time_string("12:00:00").unwrap(), expected);
    }

    #[test]
    fn test_parse_time_string_with_minutes_seconds() {
        let expected = (12 * 3600 + 30 * 60 + 45) * 1_000_000_000_i64;
        assert_eq!(Time64NanosecondBuilderWrapper::parse_time_string("12:30:45").unwrap(), expected);
    }

    #[test]
    fn test_parse_time_string_end_of_day() {
        let expected = (23 * 3600 + 59 * 60 + 59) * 1_000_000_000_i64;
        assert_eq!(Time64NanosecondBuilderWrapper::parse_time_string("23:59:59").unwrap(), expected);
    }

    #[test]
    fn test_parse_time_string_invalid_format() {
        let result = Time64NanosecondBuilderWrapper::parse_time_string("12-30-45");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_time_string_invalid_hour() {
        let result = Time64NanosecondBuilderWrapper::parse_time_string("XX:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_time_string_invalid_minute() {
        let result = Time64NanosecondBuilderWrapper::parse_time_string("12:XX:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_time_string_invalid_second() {
        let result = Time64NanosecondBuilderWrapper::parse_time_string("12:00:XX");
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // parse_datetime_string Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_datetime_string_epoch() {
        assert_eq!(
            TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T00:00:00").unwrap(),
            0
        );
    }

    #[test]
    fn test_parse_datetime_string_with_fractional_seconds() {
        // 0.1 seconds = 100M nanoseconds
        assert_eq!(
            TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T00:00:00.1000000").unwrap(),
            100_000_000
        );
    }

    #[test]
    fn test_parse_datetime_string_with_milliseconds() {
        // 0.5 seconds = 500M nanoseconds
        assert_eq!(
            TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T00:00:00.5000000").unwrap(),
            500_000_000
        );
    }

    #[test]
    fn test_parse_datetime_string_invalid_format_no_t() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01 00:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_date() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970/01/01T00:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_year() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("XXXX-01-01T00:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_month() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-XX-01T00:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_day() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-XXT00:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_time() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T00-00-00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_hour() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01TXX:00:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_minute() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T00:XX:00");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_invalid_second() {
        let result = TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T00:00:XX");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_datetime_string_one_day_after_epoch() {
        let expected = 86_400 * 1_000_000_000_i64; // 1 day in nanoseconds
        assert_eq!(
            TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-02T00:00:00").unwrap(),
            expected
        );
    }

    #[test]
    fn test_parse_datetime_string_one_hour_after_epoch() {
        let expected = 3_600 * 1_000_000_000_i64; // 1 hour in nanoseconds
        assert_eq!(
            TimestampNanosecondBuilderWrapper::parse_datetime_string("1970-01-01T01:00:00").unwrap(),
            expected
        );
    }
}
