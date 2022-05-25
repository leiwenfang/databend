// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::sync::Arc;

use common_ast::ast::ExplainKind;
use common_datablocks::DataBlock;
use common_datavalues::prelude::*;
use common_exception::ErrorCode;
use common_exception::Result;
use common_planners::ExplainPlan;
use common_planners::ExplainType;
use common_streams::DataBlockStream;
use common_streams::SendableDataBlockStream;

use crate::interpreters::plan_schedulers;
use crate::interpreters::Interpreter;
use crate::interpreters::InterpreterPtr;
use crate::optimizers::Optimizers;
use crate::pipelines::processors::PipelineBuilder;
use crate::sessions::QueryContext;
use crate::sql::optimizer::SExpr;
use crate::sql::BindContext;
use crate::sql::Metadata;
use crate::sql::MetadataRef;
use crate::sql::Planner;

pub struct ExplainInterpreterV2 {
    ctx: Arc<QueryContext>,
    query: String,
    schema: DataSchemaRef,
}

#[async_trait::async_trait]
impl Interpreter for ExplainInterpreterV2 {
    fn name(&self) -> &str {
        "ExplainInterpreterV2"
    }

    async fn execute(
        &self,
        _input_stream: Option<SendableDataBlockStream>,
    ) -> Result<SendableDataBlockStream> {
        let mut planner = Planner::new(self.ctx.clone());
        let (plan, bind_context, metadata) = planner.build_sexpr(self.query.as_str()).await?;
        let block = match bind_context
            .explain_kind
            .as_ref()
            .ok_or_else(|| ErrorCode::LogicalError("explain kind shouldn't be none"))?
        {
            ExplainKind::Syntax => self.explain_syntax(plan, metadata.clone()).await?,
            ExplainKind::Graph => todo!(),
            ExplainKind::Pipeline => todo!(),
        };
        Ok(Box::pin(DataBlockStream::create(
            self.schema.clone(),
            None,
            vec![block],
        )))
    }

    async fn start(&self) -> Result<()> {
        Ok(())
    }

    async fn finish(&self) -> Result<()> {
        Ok(())
    }
}

impl ExplainInterpreterV2 {
    pub fn try_create(ctx: Arc<QueryContext>, query: &str) -> Result<InterpreterPtr> {
        let data_field = DataField::new("explain", DataTypeImpl::String(StringType::default()));
        let schema = DataSchemaRefExt::create(vec![data_field]);
        Ok(Arc::new(ExplainInterpreterV2 {
            ctx,
            query: query.to_string(),
            schema,
        }))
    }

    pub async fn explain_syntax(&self, plan: SExpr, metadata: MetadataRef) -> Result<DataBlock> {
        let result = plan.to_format_tree(&metadata.clone()).format_indent()?;
        let formatted_plan = Series::from_data(vec![result]);
        Ok(DataBlock::create(self.schema.clone(), vec![formatted_plan]))
    }
}
