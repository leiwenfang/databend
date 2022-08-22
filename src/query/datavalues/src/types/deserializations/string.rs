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

use std::io::Read;

use common_exception::ErrorCode;
use common_exception::Result;
use common_io::prelude::*;

use crate::prelude::*;

pub struct StringDeserializer {
    pub buffer: Vec<u8>,
    pub builder: MutableStringColumn,
}

impl StringDeserializer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::new(),
            builder: MutableStringColumn::with_capacity(capacity),
        }
    }
}

impl TypeDeserializer for StringDeserializer {
    // See GroupHash.rs for StringColumn
    #[allow(clippy::uninit_vec)]
    fn de_binary(&mut self, reader: &mut &[u8], _format: &FormatSettings) -> Result<()> {
        let offset: u64 = reader.read_uvarint()?;

        self.buffer.clear();
        self.buffer.reserve(offset as usize);
        unsafe {
            self.buffer.set_len(offset as usize);
        }

        reader.read_exact(&mut self.buffer)?;
        self.builder.append_value(&self.buffer);
        Ok(())
    }

    fn de_default(&mut self, _format: &FormatSettings) {
        self.builder.append_value("");
    }

    fn de_fixed_binary_batch(
        &mut self,
        reader: &[u8],
        step: usize,
        rows: usize,
        _format: &FormatSettings,
    ) -> Result<()> {
        for row in 0..rows {
            let reader = &reader[step * row..];
            self.builder.append_value(reader);
        }
        Ok(())
    }

    fn de_json(&mut self, value: &serde_json::Value, _format: &FormatSettings) -> Result<()> {
        match value {
            serde_json::Value::String(s) => {
                self.builder.append_value(s);
                Ok(())
            }
            _ => Err(ErrorCode::BadBytes("Incorrect json value, must be string")),
        }
    }

    fn de_whole_text(&mut self, reader: &[u8], _format: &FormatSettings) -> Result<()> {
        self.builder.append_value(reader);
        Ok(())
    }

    fn de_text<R: BufferRead>(
        &mut self,
        reader: &mut NestedCheckpointReader<R>,
        _format: &FormatSettings,
    ) -> Result<()> {
        self.buffer.clear();
        reader.read_escaped_string_text(&mut self.buffer)?;
        self.builder.append_value(self.buffer.as_slice());
        Ok(())
    }

    fn de_text_quoted<R: BufferRead>(
        &mut self,
        reader: &mut NestedCheckpointReader<R>,
        _format: &FormatSettings,
    ) -> Result<()> {
        self.buffer.clear();
        reader.read_quoted_text(&mut self.buffer, b'\'')?;
        self.builder.append_value(self.buffer.as_slice());
        Ok(())
    }

    fn de_text_csv<R: BufferRead>(
        &mut self,
        reader: &mut NestedCheckpointReader<R>,
        settings: &FormatSettings,
    ) -> Result<()> {
        self.buffer.clear();
        reader.read_csv_string(&mut self.buffer, settings)?;
        if self.buffer.is_empty() {
            self.builder.append_default();
        } else {
            self.builder.append_value(self.buffer.as_slice());
        }
        Ok(())
    }

    fn append_data_value(&mut self, value: DataValue, _format: &FormatSettings) -> Result<()> {
        self.builder.append_data_value(value)
    }

    fn pop_data_value(&mut self) -> Result<DataValue> {
        self.builder.pop_data_value()
    }

    fn finish_to_column(&mut self) -> ColumnRef {
        self.builder.to_column()
    }
}