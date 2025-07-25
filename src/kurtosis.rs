use datafusion::arrow::array::{Float64Array, UInt64Array};
use datafusion::{arrow, common, error, logical_expr, scalar};
use std::{any, fmt};

make_udaf_expr_and_func!(
    KurtosisFunction,
    kurtosis,
    x,
    "Calculates the excess kurtosis (Fisher’s definition) with bias correction according to the sample size.",
    kurtosis_udaf
);

pub struct KurtosisFunction {
    signature: logical_expr::Signature,
}

impl fmt::Debug for KurtosisFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KurtosisFunction")
            .field("signature", &self.signature)
            .finish()
    }
}

impl Default for KurtosisFunction {
    fn default() -> Self {
        Self::new()
    }
}

impl KurtosisFunction {
    pub fn new() -> Self {
        Self {
            signature: logical_expr::Signature::exact(
                vec![arrow::datatypes::DataType::Float64],
                logical_expr::Volatility::Immutable,
            ),
        }
    }
}

impl logical_expr::AggregateUDFImpl for KurtosisFunction {
    fn as_any(&self) -> &dyn any::Any {
        self
    }

    fn name(&self) -> &str {
        "kurtosis"
    }

    fn signature(&self) -> &logical_expr::Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[arrow::datatypes::DataType]) -> error::Result<arrow::datatypes::DataType> {
        Ok(arrow::datatypes::DataType::Float64)
    }

    fn accumulator(
        &self,
        _acc_args: logical_expr::function::AccumulatorArgs,
    ) -> error::Result<Box<dyn logical_expr::Accumulator>> {
        Ok(Box::new(KurtosisAccumulator::new()))
    }

    fn state_fields(
        &self,
        _args: logical_expr::function::StateFieldsArgs,
    ) -> error::Result<Vec<arrow::datatypes::FieldRef>> {
        Ok(vec![
            arrow::datatypes::Field::new("count", arrow::datatypes::DataType::UInt64, true).into(),
            arrow::datatypes::Field::new("sum", arrow::datatypes::DataType::Float64, true).into(),
            arrow::datatypes::Field::new("sum_sqr", arrow::datatypes::DataType::Float64, true).into(),
            arrow::datatypes::Field::new("sum_cub", arrow::datatypes::DataType::Float64, true).into(),
            arrow::datatypes::Field::new("sum_four", arrow::datatypes::DataType::Float64, true).into(),
        ])
    }
}

/// Accumulator for calculating the excess kurtosis (Fisher’s definition) with bias correction according to the sample size.
/// This implementation follows the [DuckDB implementation]:
/// <https://github.com/duckdb/duckdb/blob/main/src/core_functions/aggregate/distributive/kurtosis.cpp>
#[derive(Debug, Default)]
pub struct KurtosisAccumulator {
    count: u64,
    sum: f64,
    sum_sqr: f64,
    sum_cub: f64,
    sum_four: f64,
}

impl KurtosisAccumulator {
    pub fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            sum_sqr: 0.0,
            sum_cub: 0.0,
            sum_four: 0.0,
        }
    }
}

impl logical_expr::Accumulator for KurtosisAccumulator {
    fn update_batch(&mut self, values: &[arrow::array::ArrayRef]) -> error::Result<()> {
        let array = common::cast::as_float64_array(&values[0])?;
        for value in array.iter().flatten() {
            self.count += 1;
            self.sum += value;
            self.sum_sqr += value.powi(2);
            self.sum_cub += value.powi(3);
            self.sum_four += value.powi(4);
        }
        Ok(())
    }

    fn merge_batch(&mut self, states: &[arrow::array::ArrayRef]) -> error::Result<()> {
        let counts = common::downcast_value!(states[0], UInt64Array);
        let sums = common::downcast_value!(states[1], Float64Array);
        let sum_sqrs = common::downcast_value!(states[2], Float64Array);
        let sum_cubs = common::downcast_value!(states[3], Float64Array);
        let sum_fours = common::downcast_value!(states[4], Float64Array);

        for i in 0..counts.len() {
            let c = counts.value(i);
            if c == 0 {
                continue;
            }
            self.count += c;
            self.sum += sums.value(i);
            self.sum_sqr += sum_sqrs.value(i);
            self.sum_cub += sum_cubs.value(i);
            self.sum_four += sum_fours.value(i);
        }

        Ok(())
    }

    fn evaluate(&mut self) -> error::Result<scalar::ScalarValue> {
        if self.count <= 3 {
            return Ok(scalar::ScalarValue::Float64(None));
        }

        let count_64 = 1_f64 / self.count as f64;
        let m4 = count_64
            * (self.sum_four - 4.0 * self.sum_cub * self.sum * count_64
                + 6.0 * self.sum_sqr * self.sum.powi(2) * count_64.powi(2)
                - 3.0 * self.sum.powi(4) * count_64.powi(3));

        let m2 = (self.sum_sqr - self.sum.powi(2) * count_64) * count_64;
        if m2 <= 0.0 {
            return Ok(scalar::ScalarValue::Float64(None));
        }

        let count = self.count as f64;
        let numerator = (count - 1.0) * ((count + 1.0) * m4 / m2.powi(2) - 3.0 * (count - 1.0));
        let denominator = (count - 2.0) * (count - 3.0);

        let target = numerator / denominator;

        Ok(scalar::ScalarValue::Float64(Some(target)))
    }

    fn size(&self) -> usize {
        std::mem::size_of_val(self)
    }

    fn state(&mut self) -> error::Result<Vec<scalar::ScalarValue>> {
        Ok(vec![
            scalar::ScalarValue::from(self.count),
            scalar::ScalarValue::from(self.sum),
            scalar::ScalarValue::from(self.sum_sqr),
            scalar::ScalarValue::from(self.sum_cub),
            scalar::ScalarValue::from(self.sum_four),
        ])
    }
}
