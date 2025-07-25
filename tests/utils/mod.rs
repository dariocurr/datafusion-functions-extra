use datafusion::{arrow, error, execution, sql};
use datafusion_functions_extra::register_all_extra_functions;
use log::debug;

pub struct TestExecution {
    ctx: execution::context::SessionContext,
}

impl TestExecution {
    pub async fn new() -> error::Result<Self> {
        let config = execution::config::SessionConfig::new();
        let mut ctx = execution::context::SessionContext::new_with_config(config);
        register_all_extra_functions(&mut ctx)?;
        Ok(Self { ctx })
    }

    pub async fn with_setup(self, sql: &str) -> Self {
        debug!("Running setup query: {sql}");
        let statements = sql::parser::DFParser::parse_sql(sql).expect("Error parsing setup query");
        for statement in statements {
            debug!("Running setup statement: {statement}");
            let statement_sql = statement.to_string();
            self.ctx
                .sql(&statement_sql)
                .await
                .expect("Error planning setup failed")
                .collect()
                .await
                .expect("Error executing setup query");
        }
        self
    }

    pub async fn run(&mut self, sql: &str) -> error::Result<Vec<arrow::record_batch::RecordBatch>> {
        debug!("Running query: {sql}");
        self.ctx.sql(sql).await?.collect().await
    }

    pub async fn run_and_format(&mut self, sql: &str) -> Vec<String> {
        let results = self.run(sql).await.expect("Error running query");
        format_results(&results)
    }
}

fn format_results(results: &[arrow::record_batch::RecordBatch]) -> Vec<String> {
    let formatted = arrow::util::pretty::pretty_format_batches(results).unwrap().to_string();

    formatted.lines().map(|s| s.to_string()).collect()
}
