// (C) Copyright IBM Corp. 2024.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use thiserror::Error;

use crate::models::ConfigValue;
use crate::models::{ConfigValueConversionError, Segment, SegmentRule};
use crate::Value;

#[derive(Debug, Error)]
pub(crate) enum SegmentEvaluationError {
    #[error(transparent)]
    SegmentEvaluationFailed(#[from] SegmentEvaluationErrorKind),

    #[error("An invalid value was encountered during rule evaluation")]
    InvalidValue {
        segment_id: String,
        segment_rule: SegmentRule,
        value: ConfigValue,
        source: ConfigValueConversionError,
    },

    #[error("Segment ID '{0}' not found")]
    SegmentIdNotFound(String),
}

#[derive(Debug, Error)]
#[error("Operation '{}' '{}' '{}' failed to evaluate: {}", segment_rule.attribute_name, segment_rule.operator,  value, source)]
pub(crate) struct SegmentEvaluationErrorKind {
    pub(crate) segment_id: String,
    pub(crate) segment_rule: SegmentRule,
    pub(crate) value: Value,
    pub(crate) source: CheckOperatorErrorDetail,
}

impl From<(CheckOperatorErrorDetail, &Segment, &SegmentRule, &Value)> for SegmentEvaluationError {
    fn from(value: (CheckOperatorErrorDetail, &Segment, &SegmentRule, &Value)) -> Self {
        let (source, segment, segment_rule, value) = value;
        Self::SegmentEvaluationFailed(SegmentEvaluationErrorKind {
            segment_id: segment.segment_id.clone(),
            segment_rule: segment_rule.clone(),
            value: value.clone(),
            source,
        })
    }
}

impl
    From<(
        ConfigValueConversionError,
        &Segment,
        &SegmentRule,
        &ConfigValue,
    )> for SegmentEvaluationError
{
    fn from(
        value: (
            ConfigValueConversionError,
            &Segment,
            &SegmentRule,
            &ConfigValue,
        ),
    ) -> Self {
        let (source, segment, segment_rule, value) = value;
        Self::InvalidValue {
            segment_id: segment.segment_id.clone(),
            segment_rule: segment_rule.clone(),
            value: value.clone(),
            source,
        }
    }
}

impl From<crate::models::ConfigValueConversionError> for SegmentEvaluationError {
    fn from(value: crate::models::ConfigValueConversionError) -> Self {
        todo!()
    }
}

#[derive(Debug, Error)]
pub(crate) enum CheckOperatorErrorDetail {
    #[error("Entity attribute is not a string.")]
    StringExpected,

    #[error("Entity attribute has unexpected type: Boolean.")]
    BooleanExpected(#[from] std::str::ParseBoolError),

    #[error("Entity attribute has unexpected type: Number.")]
    NumberExpected(#[from] serde_json::Error),

    #[error("Entity attribute is not a number.")]
    EntityAttrNotANumber,

    #[error("Operator not implemented.")]
    OperatorNotImplemented,

    #[error("Operands have different types.")]
    OperandsHaveDifferentTypes,
}
