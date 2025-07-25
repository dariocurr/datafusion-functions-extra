#![deny(warnings)]

use datafusion::{common as df_common, error, execution, logical_expr};
use log::debug;
use mode::mode_udaf;
use std::sync;

#[macro_use]
pub mod macros;
pub mod common;
pub mod kurtosis;
pub mod kurtosis_pop;
pub mod max_min_by;
pub mod mode;
pub mod skewness;
pub mod expr_extra_fn {
    pub use super::kurtosis::kurtosis;
    pub use super::kurtosis_pop::kurtosis_pop;
    pub use super::max_min_by::max_by;
    pub use super::max_min_by::min_by;
    pub use super::mode::mode;
    pub use super::skewness::skewness;
}

pub fn all_extra_aggregate_functions() -> Vec<sync::Arc<logical_expr::AggregateUDF>> {
    vec![
        mode_udaf(),
        max_min_by::max_by_udaf(),
        max_min_by::min_by_udaf(),
        kurtosis::kurtosis_udaf(),
        skewness::skewness_udaf(),
        kurtosis_pop::kurtosis_pop_udaf(),
    ]
}

/// Registers all enabled packages with a [`FunctionRegistry`]
pub fn register_all_extra_functions(registry: &mut dyn execution::FunctionRegistry) -> df_common::Result<()> {
    let functions: Vec<sync::Arc<logical_expr::AggregateUDF>> = all_extra_aggregate_functions();

    functions.into_iter().try_for_each(|udf| {
        let existing_udaf = registry.register_udaf(udf)?;
        if let Some(existing_udaf) = existing_udaf {
            debug!("Overwrite existing UDAF: {}", existing_udaf.name());
        }
        Ok(()) as error::Result<()>
    })?;

    Ok(())
}
