//  Copyright 2021 Datafuse Labs.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//

use std::sync::Arc;

use common_base::tokio;
use common_datablocks::DataBlock;
use common_datavalues::prelude::SeriesFrom;
use common_datavalues::series::Series;
use common_datavalues::DataField;
use common_datavalues::DataSchemaRefExt;
use common_datavalues::DataType;
use tempfile::TempDir;

use crate::datasources::table::fuse::io::BlockAppender;
use crate::datasources::table::fuse::DEFAULT_CHUNK_BLOCK_NUM;

#[tokio::test]
async fn test_fuse_table_block_appender() {
    let tmp_dir = TempDir::new().unwrap();
    let local_fs = common_dal::Local::with_path(tmp_dir.path().to_owned());
    let local_fs = Arc::new(local_fs);
    let schema = DataSchemaRefExt::create(vec![DataField::new("a", DataType::Int32, false)]);

    // single segments
    let block = DataBlock::create_by_array(schema.clone(), vec![Series::new(vec![1, 2, 3])]);
    let block_stream = futures::stream::iter(vec![Ok(block)]);
    let r = BlockAppender::append_blocks(
        local_fs.clone(),
        Box::pin(block_stream),
        schema.as_ref(),
        DEFAULT_CHUNK_BLOCK_NUM,
        0,
    )
    .await;
    assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
    let r = r.unwrap();
    assert_eq!(r.len(), 1);

    // multiple segments
    let number_of_blocks = 30;
    let chunk_size = 10;
    let block = DataBlock::create_by_array(schema.clone(), vec![Series::new(vec![1, 2, 3])]);
    let blocks = std::iter::repeat(Ok(block)).take(number_of_blocks);
    let block_stream = futures::stream::iter(blocks);
    let r = BlockAppender::append_blocks(
        local_fs.clone(),
        Box::pin(block_stream),
        schema.as_ref(),
        chunk_size,
        0,
    )
    .await;
    assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
    let r = r.unwrap();
    assert_eq!(r.len(), number_of_blocks / chunk_size);

    // empty blocks
    let block_stream = futures::stream::iter(vec![]);
    let r = BlockAppender::append_blocks(
        local_fs,
        Box::pin(block_stream),
        schema.as_ref(),
        DEFAULT_CHUNK_BLOCK_NUM,
        0,
    )
    .await;
    assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
    assert!(r.unwrap().is_empty())
}

#[test]
fn test_fuse_table_block_appender_reshape() {
    let schema = DataSchemaRefExt::create(vec![DataField::new("a", DataType::Int32, false)]);

    // 1 empty blocks
    // 1.1 empty block, zero block_size_threshold
    let blocks = vec![];
    let r = BlockAppender::reshape_blocks(blocks, 0);
    assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
    let r = r.unwrap();
    assert_eq!(r.len(), 0);

    // 1.2 empty block, non block_size_threshold
    let blocks = vec![];
    let r = BlockAppender::reshape_blocks(blocks, 100);
    assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
    let r = r.unwrap();
    assert_eq!(r.len(), 0);

    // 2. merge
    // 2.1 exactly into on block
    {
        let block = DataBlock::create_by_array(schema.clone(), vec![Series::new(vec![1, 2, 3])]);
        let block_size = block.memory_size();
        let block_num = 10;
        let blocks = std::iter::repeat(block).take(block_num);
        let block_size_threshold = block_size * block_num;
        let r = BlockAppender::reshape_blocks(blocks.collect(), block_size_threshold);
        assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].memory_size(), block_size_threshold);
    }

    // 2.1 with remainders
    {
        let block = DataBlock::create_by_array(schema.clone(), vec![Series::new(vec![1, 2, 3])]);
        let block_size = block.memory_size();
        let block_num = 10;
        let blocks = std::iter::repeat(block.clone()).take(block_num);
        let block_size_threshold = block_size * block_num;

        // append an extra block
        let blocks = blocks.chain(std::iter::once(block));
        let r = BlockAppender::reshape_blocks(blocks.collect(), block_size_threshold);
        assert!(r.is_ok(), "oops, unexpected result: {:?}", r);
        let r = r.unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].memory_size(), block_size_threshold);
        assert_eq!(r[1].memory_size(), block_size);
    }
}
