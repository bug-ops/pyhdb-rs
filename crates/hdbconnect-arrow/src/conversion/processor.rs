//! Batch processor for streaming conversion of HANA rows to `RecordBatch`es.
//!
//! Implements buffered batch creation with configurable batch size.

use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::{DataType, Schema, SchemaRef, TimeUnit};

use crate::Result;
use crate::builders::dispatch::{BuilderEnum, BuilderKind};
use crate::builders::factory::BuilderFactory;
use crate::traits::builder::HanaCompatibleBuilder;
use crate::traits::row::RowLike;
use crate::traits::streaming::BatchConfig;

/// Append a HANA value to a builder, handling NULL values.
#[inline]
fn append_value_to_builder<B: HanaCompatibleBuilder>(
    builder: &mut B,
    value: &hdbconnect::HdbValue,
) -> Result<()> {
    match value {
        hdbconnect::HdbValue::NULL => {
            builder.append_null();
            Ok(())
        }
        v => builder.append_hana_value(v),
    }
}

/// Macro to generate specialized homogeneous processing loops for inline variants.
///
/// Eliminates per-value enum dispatch by extracting concrete builder type per column.
macro_rules! specialize_homogeneous_loop {
    ($self:expr, $row:expr, $variant:ident) => {{
        for (i, builder) in $self.builders.iter_mut().enumerate() {
            let BuilderEnum::$variant(concrete_builder) = builder else {
                unreachable!("SchemaProfile guarantees homogeneous type")
            };
            append_value_to_builder(concrete_builder, &$row.get(i))?;
        }
        Ok(())
    }};
}

/// Macro for boxed variants that require explicit deref.
macro_rules! specialize_homogeneous_loop_boxed {
    ($self:expr, $row:expr, $variant:ident) => {{
        for (i, builder) in $self.builders.iter_mut().enumerate() {
            let BuilderEnum::$variant(boxed) = builder else {
                unreachable!("SchemaProfile guarantees homogeneous type")
            };
            append_value_to_builder(boxed.as_mut(), &$row.get(i))?;
        }
        Ok(())
    }};
}

/// Schema profile for processor optimization.
///
/// Classifies schemas as homogeneous (all columns same type) or mixed
/// to enable specialized processing paths.
#[derive(Debug, Clone)]
pub enum SchemaProfile {
    /// All columns share the same type.
    Homogeneous {
        /// Number of columns in the schema.
        column_count: usize,
        /// The common builder kind for all columns.
        kind: BuilderKind,
    },
    /// Columns have different types.
    Mixed,
}

impl SchemaProfile {
    /// Analyze schema and return its profile.
    #[must_use]
    pub fn analyze(schema: &Schema) -> Self {
        let fields = schema.fields();
        if fields.is_empty() {
            return Self::Mixed;
        }

        let first_type = fields[0].data_type();
        let all_same = fields
            .iter()
            .skip(1)
            .all(|f| Self::types_equivalent(first_type, f.data_type()));

        // Verify discriminant comparison works correctly for homogeneous schemas
        debug_assert!(
            !all_same
                || fields.iter().all(|f| {
                    let kind1 = Self::data_type_to_kind(first_type);
                    let kind2 = Self::data_type_to_kind(f.data_type());
                    kind1 == kind2
                }),
            "Discriminant-equivalent types must map to same BuilderKind"
        );

        if all_same {
            Self::Homogeneous {
                column_count: fields.len(),
                kind: Self::data_type_to_kind(first_type),
            }
        } else {
            Self::Mixed
        }
    }

    /// Check if two `DataTypes` are equivalent for homogeneous schema detection.
    ///
    /// Uses discriminant comparison, which treats types as equivalent if they
    /// have the same variant, ignoring associated data. For example:
    /// - `Decimal128(18, 2)` is equivalent to `Decimal128(10, 4)`
    /// - `Timestamp(Nanosecond, None)` is equivalent to `Timestamp(Nanosecond, Some(tz))`
    ///
    /// This design choice enables homogeneous optimization for schemas where
    /// all columns use the same Arrow type variant, even if parameters differ.
    fn types_equivalent(a: &DataType, b: &DataType) -> bool {
        std::mem::discriminant(a) == std::mem::discriminant(b)
    }

    #[allow(clippy::match_same_arms, clippy::missing_const_for_fn)]
    fn data_type_to_kind(dt: &DataType) -> BuilderKind {
        match dt {
            DataType::UInt8 => BuilderKind::UInt8,
            DataType::Int16 => BuilderKind::Int16,
            DataType::Int32 => BuilderKind::Int32,
            DataType::Int64 => BuilderKind::Int64,
            DataType::Float32 => BuilderKind::Float32,
            DataType::Float64 => BuilderKind::Float64,
            DataType::Decimal128(_, _) => BuilderKind::Decimal128,
            DataType::Boolean => BuilderKind::Boolean,
            DataType::Utf8 => BuilderKind::Utf8,
            DataType::LargeUtf8 => BuilderKind::LargeUtf8,
            DataType::Binary => BuilderKind::Binary,
            DataType::LargeBinary => BuilderKind::LargeBinary,
            DataType::FixedSizeBinary(_) => BuilderKind::FixedSizeBinary,
            DataType::Date32 => BuilderKind::Date32,
            DataType::Time64(TimeUnit::Nanosecond) => BuilderKind::Time64Nanosecond,
            DataType::Timestamp(TimeUnit::Nanosecond, None) => BuilderKind::TimestampNanosecond,
            _ => BuilderKind::Utf8,
        }
    }

    /// Returns true if the schema is homogeneous.
    #[must_use]
    pub const fn is_homogeneous(&self) -> bool {
        matches!(self, Self::Homogeneous { .. })
    }
}

/// Processor that converts HANA rows into Arrow `RecordBatch`es.
///
/// Buffers rows until `batch_size` is reached, then emits a `RecordBatch`.
/// Uses enum-based dispatch to eliminate vtable overhead.
///
/// # Example
///
/// ```rust,ignore
/// use hdbconnect_arrow::conversion::HanaBatchProcessor;
/// use hdbconnect_arrow::traits::streaming::BatchConfig;
///
/// let schema = /* Arrow schema */;
/// let config = BatchConfig::with_batch_size(10000);
/// let mut processor = HanaBatchProcessor::new(Arc::new(schema), config);
///
/// for row in result_set {
///     if let Some(batch) = processor.process_row(row)? {
///         // Process batch
///     }
/// }
///
/// // Don't forget to flush remaining rows
/// if let Some(batch) = processor.flush()? {
///     // Process final batch
/// }
/// ```
pub struct HanaBatchProcessor {
    schema: SchemaRef,
    config: BatchConfig,
    builders: Vec<BuilderEnum>,
    profile: SchemaProfile,
    row_count: usize,
}

impl std::fmt::Debug for HanaBatchProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HanaBatchProcessor")
            .field("schema", &self.schema)
            .field("config", &self.config)
            .field("builders", &format!("[{} builders]", self.builders.len()))
            .field("profile", &self.profile)
            .field("row_count", &self.row_count)
            .finish()
    }
}

impl HanaBatchProcessor {
    /// Create a new batch processor.
    ///
    /// # Arguments
    ///
    /// * `schema` - Arrow schema for the batches
    /// * `config` - Batch processing configuration
    #[must_use]
    pub fn new(schema: SchemaRef, config: BatchConfig) -> Self {
        let factory = BuilderFactory::from_config(&config);
        let builders = factory.create_builders_enum_for_schema_with_metadata(&schema);
        let profile = SchemaProfile::analyze(&schema);

        Self {
            schema,
            config,
            builders,
            profile,
            row_count: 0,
        }
    }

    /// Create with default configuration.
    #[must_use]
    pub fn with_defaults(schema: SchemaRef) -> Self {
        Self::new(schema, BatchConfig::default())
    }

    /// Process a single row.
    ///
    /// Returns `Ok(Some(batch))` when a batch is ready, `Ok(None)` when more
    /// rows are needed to fill a batch.
    ///
    /// # Errors
    ///
    /// Returns error if value conversion fails or schema mismatches.
    pub fn process_row(&mut self, row: &hdbconnect::Row) -> Result<Option<RecordBatch>> {
        self.process_row_generic(row)
    }

    /// Process a single row using the generic `RowLike` trait.
    ///
    /// This method enables unit testing with `MockRow` instead of requiring
    /// a HANA connection.
    ///
    /// Returns `Ok(Some(batch))` when a batch is ready, `Ok(None)` when more
    /// rows are needed to fill a batch.
    ///
    /// # Errors
    ///
    /// Returns error if value conversion fails or schema mismatches.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use hdbconnect_arrow::traits::row::{MockRow, MockRowBuilder};
    ///
    /// let row = MockRowBuilder::new().int(42).string("test").build();
    /// let result = processor.process_row_generic(&row)?;
    /// ```
    pub fn process_row_generic<R: RowLike>(&mut self, row: &R) -> Result<Option<RecordBatch>> {
        if row.len() != self.builders.len() {
            return Err(crate::ArrowConversionError::schema_mismatch(
                self.builders.len(),
                row.len(),
            ));
        }

        match &self.profile {
            SchemaProfile::Homogeneous { kind, .. } => {
                self.process_row_homogeneous(row, *kind)?;
            }
            SchemaProfile::Mixed => {
                self.process_row_mixed(row)?;
            }
        }

        self.row_count += 1;

        if self.row_count >= self.config.batch_size.get() {
            return Ok(Some(self.finish_current_batch()?));
        }

        Ok(None)
    }

    /// Process row for homogeneous schemas with specialized dispatch.
    ///
    /// Hoists type match outside the column loop, creating monomorphized
    /// inner loops with zero enum dispatch overhead. Specialized for the
    /// top 5 most common HANA types (Int64, Decimal128, Utf8, Int32, Float64).
    ///
    /// Other types fall back to generic enum dispatch path.
    fn process_row_homogeneous<R: RowLike>(&mut self, row: &R, kind: BuilderKind) -> Result<()> {
        match kind {
            BuilderKind::Int64 => {
                specialize_homogeneous_loop!(self, row, Int64)
            }
            BuilderKind::Decimal128 => {
                specialize_homogeneous_loop_boxed!(self, row, Decimal128)
            }
            BuilderKind::Utf8 => {
                specialize_homogeneous_loop_boxed!(self, row, Utf8)
            }
            BuilderKind::Int32 => {
                specialize_homogeneous_loop!(self, row, Int32)
            }
            BuilderKind::Float64 => {
                specialize_homogeneous_loop!(self, row, Float64)
            }
            _ => self.process_row_mixed(row),
        }
    }

    /// Process row for mixed schemas with enum dispatch per column.
    fn process_row_mixed<R: RowLike>(&mut self, row: &R) -> Result<()> {
        for (i, builder) in self.builders.iter_mut().enumerate() {
            let value = row.get(i);
            match value {
                hdbconnect::HdbValue::NULL => builder.append_null(),
                v => builder.append_hana_value(v)?,
            }
        }
        Ok(())
    }

    /// Flush any remaining rows as a final batch.
    ///
    /// # Errors
    ///
    /// Returns error if `RecordBatch` creation fails.
    pub fn flush(&mut self) -> Result<Option<RecordBatch>> {
        if self.row_count == 0 {
            return Ok(None);
        }

        Ok(Some(self.finish_current_batch()?))
    }

    /// Returns the schema of batches produced by this processor.
    #[must_use]
    pub fn schema(&self) -> SchemaRef {
        Arc::clone(&self.schema)
    }

    /// Returns the current row count in the buffer.
    #[must_use]
    pub const fn buffered_rows(&self) -> usize {
        self.row_count
    }

    /// Returns the schema profile for this processor.
    #[must_use]
    pub const fn profile(&self) -> &SchemaProfile {
        &self.profile
    }

    /// Finish the current batch and reset builders.
    ///
    /// Arrow builders reset their internal state after `finish()`, keeping
    /// allocated capacity for the next batch. This avoids heap allocations
    /// at batch boundaries.
    ///
    /// # Errors
    ///
    /// Returns error if `RecordBatch` creation fails.
    fn finish_current_batch(&mut self) -> Result<RecordBatch> {
        let arrays: Vec<_> = self.builders.iter_mut().map(BuilderEnum::finish).collect();

        let batch = RecordBatch::try_new(Arc::clone(&self.schema), arrays)
            .map_err(|e| crate::ArrowConversionError::value_conversion("batch", e.to_string()))?;

        self.row_count = 0;

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // SchemaProfile Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_profile_homogeneous_int64() {
        let schema = Schema::new(vec![
            Field::new("col1", DataType::Int64, false),
            Field::new("col2", DataType::Int64, false),
            Field::new("col3", DataType::Int64, false),
        ]);

        let profile = SchemaProfile::analyze(&schema);
        assert!(profile.is_homogeneous());
        match profile {
            SchemaProfile::Homogeneous { column_count, kind } => {
                assert_eq!(column_count, 3);
                assert_eq!(kind, BuilderKind::Int64);
            }
            SchemaProfile::Mixed => panic!("Expected homogeneous profile"),
        }
    }

    #[test]
    fn test_schema_profile_homogeneous_utf8() {
        let schema = Schema::new(vec![
            Field::new("col1", DataType::Utf8, true),
            Field::new("col2", DataType::Utf8, true),
        ]);

        let profile = SchemaProfile::analyze(&schema);
        assert!(profile.is_homogeneous());
        match profile {
            SchemaProfile::Homogeneous { column_count, kind } => {
                assert_eq!(column_count, 2);
                assert_eq!(kind, BuilderKind::Utf8);
            }
            SchemaProfile::Mixed => panic!("Expected homogeneous profile"),
        }
    }

    #[test]
    fn test_schema_profile_mixed() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("active", DataType::Boolean, false),
        ]);

        let profile = SchemaProfile::analyze(&schema);
        assert!(!profile.is_homogeneous());
        assert!(matches!(profile, SchemaProfile::Mixed));
    }

    #[test]
    fn test_schema_profile_single_column() {
        let schema = Schema::new(vec![Field::new("id", DataType::Int32, false)]);

        let profile = SchemaProfile::analyze(&schema);
        assert!(profile.is_homogeneous());
        match profile {
            SchemaProfile::Homogeneous { column_count, kind } => {
                assert_eq!(column_count, 1);
                assert_eq!(kind, BuilderKind::Int32);
            }
            SchemaProfile::Mixed => panic!("Expected homogeneous profile"),
        }
    }

    #[test]
    fn test_schema_profile_empty() {
        let fields: Vec<Field> = vec![];
        let schema = Schema::new(fields);

        let profile = SchemaProfile::analyze(&schema);
        assert!(!profile.is_homogeneous());
        assert!(matches!(profile, SchemaProfile::Mixed));
    }

    #[test]
    fn test_schema_profile_decimal_same_precision_scale() {
        let schema = Schema::new(vec![
            Field::new("price1", DataType::Decimal128(18, 2), false),
            Field::new("price2", DataType::Decimal128(18, 2), false),
        ]);

        let profile = SchemaProfile::analyze(&schema);
        assert!(profile.is_homogeneous());
    }

    #[test]
    fn test_schema_profile_decimal_different_precision() {
        let schema = Schema::new(vec![
            Field::new("price1", DataType::Decimal128(18, 2), false),
            Field::new("price2", DataType::Decimal128(10, 4), false),
        ]);

        let profile = SchemaProfile::analyze(&schema);
        assert!(profile.is_homogeneous());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Processor Creation Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_creation() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100);

        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_defaults() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_schema() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(Arc::clone(&schema));

        let returned_schema = processor.schema();
        assert_eq!(returned_schema.fields().len(), 2);
        assert_eq!(returned_schema.field(0).name(), "id");
        assert_eq!(returned_schema.field(1).name(), "name");
    }

    #[test]
    fn test_processor_initial_buffered_rows() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_profile_homogeneous() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Int64, false),
            Field::new("col2", DataType::Int64, false),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert!(processor.profile().is_homogeneous());
    }

    #[test]
    fn test_processor_profile_mixed() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert!(!processor.profile().is_homogeneous());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Processor with Different Configs
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_with_small_batch_size() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(10);
        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_large_batch_size() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100000);
        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_custom_config() {
        let schema = Arc::new(Schema::new(vec![Field::new("data", DataType::Utf8, true)]));
        let config = BatchConfig::with_batch_size(500)
            .string_capacity(10000)
            .binary_capacity(5000);
        let processor = HanaBatchProcessor::new(schema, config);
        assert_eq!(processor.buffered_rows(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Processor with Different Schema Types
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_with_empty_schema() {
        let fields: Vec<Field> = vec![];
        let schema = Arc::new(Schema::new(fields));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_single_column_schema() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_multi_column_schema() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("price", DataType::Decimal128(18, 2), false),
            Field::new("is_active", DataType::Boolean, false),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_all_numeric_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("tiny", DataType::UInt8, false),
            Field::new("small", DataType::Int16, false),
            Field::new("int", DataType::Int32, false),
            Field::new("big", DataType::Int64, false),
            Field::new("real", DataType::Float32, false),
            Field::new("double", DataType::Float64, false),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_string_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("small_str", DataType::Utf8, true),
            Field::new("large_str", DataType::LargeUtf8, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_with_binary_types() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("bin", DataType::Binary, true),
            Field::new("large_bin", DataType::LargeBinary, true),
            Field::new("fixed_bin", DataType::FixedSizeBinary(16), true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(schema);
        assert_eq!(processor.buffered_rows(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Flush Tests (without rows - tests empty flush)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_flush_empty() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let mut processor = HanaBatchProcessor::with_defaults(schema);

        let result = processor.flush();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_processor_flush_multiple_times_when_empty() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let mut processor = HanaBatchProcessor::with_defaults(schema);

        assert!(processor.flush().unwrap().is_none());
        assert!(processor.flush().unwrap().is_none());
        assert!(processor.flush().unwrap().is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Debug Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_debug() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let processor = HanaBatchProcessor::with_defaults(schema);

        let debug_str = format!("{:?}", processor);
        assert!(debug_str.contains("HanaBatchProcessor"));
        assert!(debug_str.contains("row_count"));
        assert!(debug_str.contains("builders"));
        assert!(debug_str.contains("profile"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Schema Ref Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_schema_returns_same_schema() {
        let original_schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("value", DataType::Float64, true),
        ]));
        let processor = HanaBatchProcessor::with_defaults(Arc::clone(&original_schema));

        let schema1 = processor.schema();
        let schema2 = processor.schema();

        assert!(Arc::ptr_eq(&schema1, &schema2));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MockRow Tests (unit testing without HANA connection)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_process_row_generic_with_mock_row() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(10);
        let mut processor = HanaBatchProcessor::new(schema, config);

        let row = MockRowBuilder::new().int(42).string("test").build();

        let result = processor.process_row_generic(&row);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Not enough rows for batch
        assert_eq!(processor.buffered_rows(), 1);
    }

    #[test]
    fn test_process_row_generic_batch_ready() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(3);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Add rows until batch is ready
        for i in 0..3 {
            let row = MockRowBuilder::new().int(i).build();
            let result = processor.process_row_generic(&row).unwrap();
            if i < 2 {
                assert!(result.is_none());
            } else {
                // Third row should trigger batch
                let batch = result.expect("batch should be ready");
                assert_eq!(batch.num_rows(), 3);
            }
        }
    }

    #[test]
    fn test_process_row_generic_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, true),
            Field::new("name", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Row with null values
        let row = MockRowBuilder::new().null().null().build();

        let result = processor.process_row_generic(&row);
        assert!(result.is_ok());
        assert_eq!(processor.buffered_rows(), 1);
    }

    #[test]
    fn test_process_row_generic_schema_mismatch() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let mut processor = HanaBatchProcessor::with_defaults(schema);

        // Row with wrong number of columns
        let row = MockRowBuilder::new().int(1).string("extra").build();

        let result = processor.process_row_generic(&row);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_schema_mismatch());
    }

    #[test]
    fn test_process_row_generic_flush() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let config = BatchConfig::with_batch_size(100);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Add some rows (less than batch size)
        for i in 0..5 {
            let row = MockRowBuilder::new().int(i).build();
            processor.process_row_generic(&row).unwrap();
        }

        assert_eq!(processor.buffered_rows(), 5);

        // Flush remaining rows
        let batch = processor
            .flush()
            .unwrap()
            .expect("should have remaining rows");
        assert_eq!(batch.num_rows(), 5);
        assert_eq!(processor.buffered_rows(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Builder Reuse Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_builder_reuse_after_finish() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        // Process first batch
        for i in 0..2 {
            let row = MockRowBuilder::new().int(i).string("test").build();
            let result = processor.process_row_generic(&row).unwrap();
            if i == 1 {
                assert!(result.is_some(), "First batch should be ready");
            }
        }

        // Verify processor can continue processing (builders reused)
        for i in 2..4 {
            let row = MockRowBuilder::new().int(i).string("test2").build();
            let result = processor.process_row_generic(&row).unwrap();
            if i == 3 {
                let batch = result.expect("Second batch should be ready");
                assert_eq!(batch.num_rows(), 2);
                // Verify data is from second batch, not first
                let id_array = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<arrow_array::Int32Array>()
                    .unwrap();
                assert_eq!(id_array.value(0), 2);
                assert_eq!(id_array.value(1), 3);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Homogeneous Schema Processing Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_processor_homogeneous_int64() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Int64, false),
            Field::new("col2", DataType::Int64, false),
            Field::new("col3", DataType::Int64, false),
        ]));
        let config = BatchConfig::with_batch_size(3);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        // Process rows
        processor
            .process_row_generic(&MockRowBuilder::new().bigint(1).bigint(2).bigint(3).build())
            .unwrap();
        processor
            .process_row_generic(&MockRowBuilder::new().bigint(4).bigint(5).bigint(6).build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().bigint(7).bigint(8).bigint(9).build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 3);
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_processor_homogeneous_int32() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Int32, false),
            Field::new("col2", DataType::Int32, false),
            Field::new("col3", DataType::Int32, false),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().int(10).int(20).int(30).build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().int(40).int(50).int(60).build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_processor_homogeneous_float64() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Float64, false),
            Field::new("col2", DataType::Float64, false),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().double(1.5).double(2.5).build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().double(3.5).double(4.5).build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 2);
    }

    #[test]
    fn test_processor_homogeneous_decimal128() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Decimal128(18, 2), false),
            Field::new("col2", DataType::Decimal128(18, 2), false),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(
                &MockRowBuilder::new()
                    .decimal_str("100.50")
                    .decimal_str("200.75")
                    .build(),
            )
            .unwrap();
        let result = processor
            .process_row_generic(
                &MockRowBuilder::new()
                    .decimal_str("300.25")
                    .decimal_str("400.99")
                    .build(),
            )
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 2);
    }

    #[test]
    fn test_processor_homogeneous_utf8() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Utf8, true),
            Field::new("col2", DataType::Utf8, true),
            Field::new("col3", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(
                &MockRowBuilder::new()
                    .string("alice")
                    .string("bob")
                    .string("charlie")
                    .build(),
            )
            .unwrap();
        let result = processor
            .process_row_generic(
                &MockRowBuilder::new()
                    .string("diana")
                    .string("eve")
                    .string("frank")
                    .build(),
            )
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_processor_mixed_schema() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("active", DataType::Boolean, false),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(!processor.profile().is_homogeneous());

        // Process rows
        processor
            .process_row_generic(
                &MockRowBuilder::new()
                    .bigint(1)
                    .string("Alice")
                    .boolean(true)
                    .build(),
            )
            .unwrap();
        let result = processor
            .process_row_generic(
                &MockRowBuilder::new()
                    .bigint(2)
                    .string("Bob")
                    .boolean(false)
                    .build(),
            )
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_processor_homogeneous_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Int64, true),
            Field::new("col2", DataType::Int64, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().bigint(1).null().build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().null().bigint(2).build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
    }

    #[test]
    fn test_processor_homogeneous_int32_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Int32, true),
            Field::new("col2", DataType::Int32, true),
            Field::new("col3", DataType::Int32, true),
        ]));
        let config = BatchConfig::with_batch_size(3);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().int(1).null().int(3).build())
            .unwrap();
        processor
            .process_row_generic(&MockRowBuilder::new().null().int(5).null().build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().int(7).int(8).null().build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 3);
        assert_eq!(batch.num_columns(), 3);
    }

    #[test]
    fn test_processor_homogeneous_float64_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Float64, true),
            Field::new("col2", DataType::Float64, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().double(1.5).null().build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().null().double(3.5).build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
    }

    #[test]
    fn test_processor_homogeneous_decimal128_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Decimal128(18, 2), true),
            Field::new("col2", DataType::Decimal128(18, 2), true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().decimal_str("100.50").null().build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().null().decimal_str("400.99").build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
    }

    #[test]
    fn test_processor_homogeneous_utf8_with_nulls() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Utf8, true),
            Field::new("col2", DataType::Utf8, true),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        processor
            .process_row_generic(&MockRowBuilder::new().string("hello").null().build())
            .unwrap();
        let result = processor
            .process_row_generic(&MockRowBuilder::new().null().string("world").build())
            .unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 2);
    }

    #[test]
    fn test_processor_homogeneous_wide_schema() {
        use crate::traits::row::MockRowBuilder;

        let mut fields = vec![];
        for i in 0..100 {
            fields.push(Field::new(&format!("col{}", i), DataType::Int64, false));
        }

        let schema = Arc::new(Schema::new(fields));
        let config = BatchConfig::with_batch_size(1);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        let mut row_builder = MockRowBuilder::new();
        for i in 0..100 {
            row_builder = row_builder.bigint(i as i64);
        }

        let result = processor.process_row_generic(&row_builder.build()).unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 100);
    }

    #[test]
    fn test_processor_homogeneous_unsupported_type_fallback() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Boolean, false),
            Field::new("col2", DataType::Boolean, false),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());
    }

    #[test]
    fn test_processor_multiple_batches_homogeneous() {
        use crate::traits::row::MockRowBuilder;

        let schema = Arc::new(Schema::new(vec![
            Field::new("col1", DataType::Int64, false),
            Field::new("col2", DataType::Int64, false),
        ]));
        let config = BatchConfig::with_batch_size(2);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        // First batch
        processor
            .process_row_generic(&MockRowBuilder::new().bigint(1).bigint(2).build())
            .unwrap();
        let batch1 = processor
            .process_row_generic(&MockRowBuilder::new().bigint(3).bigint(4).build())
            .unwrap();
        assert!(batch1.is_some());

        // Second batch
        processor
            .process_row_generic(&MockRowBuilder::new().bigint(5).bigint(6).build())
            .unwrap();
        let batch2 = processor
            .process_row_generic(&MockRowBuilder::new().bigint(7).bigint(8).build())
            .unwrap();
        assert!(batch2.is_some());

        // Verify both batches
        assert_eq!(processor.buffered_rows(), 0);
    }

    #[test]
    fn test_processor_homogeneous_int32_wide() {
        use crate::traits::row::MockRowBuilder;

        let mut fields = vec![];
        for i in 0..50 {
            fields.push(Field::new(&format!("col{}", i), DataType::Int32, false));
        }

        let schema = Arc::new(Schema::new(fields));
        let config = BatchConfig::with_batch_size(1);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        let mut row_builder = MockRowBuilder::new();
        for i in 0..50 {
            row_builder = row_builder.int(i as i32);
        }

        let result = processor.process_row_generic(&row_builder.build()).unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 50);
    }

    #[test]
    fn test_processor_homogeneous_float64_wide() {
        use crate::traits::row::MockRowBuilder;

        let mut fields = vec![];
        for i in 0..30 {
            fields.push(Field::new(&format!("col{}", i), DataType::Float64, false));
        }

        let schema = Arc::new(Schema::new(fields));
        let config = BatchConfig::with_batch_size(1);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        let mut row_builder = MockRowBuilder::new();
        for i in 0..30 {
            row_builder = row_builder.double(i as f64 * 1.5);
        }

        let result = processor.process_row_generic(&row_builder.build()).unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 30);
    }

    #[test]
    fn test_processor_homogeneous_utf8_wide() {
        use crate::traits::row::MockRowBuilder;

        let mut fields = vec![];
        for i in 0..20 {
            fields.push(Field::new(&format!("col{}", i), DataType::Utf8, true));
        }

        let schema = Arc::new(Schema::new(fields));
        let config = BatchConfig::with_batch_size(1);
        let mut processor = HanaBatchProcessor::new(schema, config);

        assert!(processor.profile().is_homogeneous());

        let mut row_builder = MockRowBuilder::new();
        for i in 0..20 {
            row_builder = row_builder.string(&format!("value{}", i));
        }

        let result = processor.process_row_generic(&row_builder.build()).unwrap();

        let batch = result.expect("batch should be ready");
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 20);
    }
}
