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

pub(crate) mod errors;

use std::collections::HashMap;

use crate::entity::Entity;
use crate::errors::Result;
use crate::models::Segment;
use crate::models::TargetingRule;
use crate::Value;
use errors::{CheckOperatorErrorDetail, SegmentEvaluationError};

pub(crate) fn find_applicable_segment_rule_for_entity(
    segments: &HashMap<String, Segment>,
    segment_rules: impl Iterator<Item = TargetingRule>,
    entity: &impl Entity,
) -> Result<Option<TargetingRule>> {
    let mut targeting_rules = segment_rules.collect::<Vec<_>>();
    targeting_rules.sort_by(|a, b| a.order.cmp(&b.order));

    for targeting_rule in targeting_rules.into_iter() {
        if targeting_rule_applies_to_entity(segments, &targeting_rule, entity)? {
            return Ok(Some(targeting_rule));
        }
    }
    Ok(None)
}

fn targeting_rule_applies_to_entity(
    segments: &HashMap<String, Segment>,
    targeting_rule: &TargetingRule,
    entity: &impl Entity,
) -> std::result::Result<bool, SegmentEvaluationError> {
    // TODO: we need to get the naming correct here to distinguish between rules, segments, segment_ids, targeting_rules etc. correctly
    let rules = &targeting_rule.rules;
    for rule in rules.iter() {
        let rule_applies = segment_applies_to_entity(segments, &rule.segments, entity)?;
        if rule_applies {
            return Ok(true);
        }
    }
    Ok(false)
}

fn segment_applies_to_entity(
    segments: &HashMap<String, Segment>,
    segment_ids: &[String],
    entity: &impl Entity,
) -> std::result::Result<bool, SegmentEvaluationError> {
    for segment_id in segment_ids.iter() {
        let segment = segments
            .get(segment_id)
            .ok_or(SegmentEvaluationError::SegmentIdNotFound(
                segment_id.clone(),
            ))?;
        let applies = belong_to_segment(segment, entity.get_attributes())?;
        if applies {
            return Ok(true);
        }
    }
    Ok(false)
}

fn belong_to_segment(
    segment: &Segment,
    attrs: HashMap<String, Value>,
) -> std::result::Result<bool, SegmentEvaluationError> {
    for rule in segment.rules.iter() {
        let operator = &rule.operator;
        let attr_name = &rule.attribute_name;
        let attr_value = attrs.get(attr_name);
        if attr_value.is_none() {
            return Ok(false);
        }
        let rule_result = match attr_value {
            None => {
                println!("Warning: Operation '{attr_name}' '{operator}' '[...]' failed to evaluate: '{attr_name}' not found in entity");
                false
            }
            Some(attr_value) => {
                // FIXME: the following algorithm is too hard to read. Is it just me or do we need to simplify this?
                // One of the values needs to match.
                // Find a candidate (a candidate corresponds to a value which matches or which might match but the operator failed):
                let candidate = rule
                    .values
                    .iter()
                    .find_map(|value| {
                        //let value: Value = value.try_into().unwrap();
                        match value.try_into() {
                            Err(e) => Some(Err((e, segment, rule, value).into())),
                            Ok(value) => match check_operator(attr_value, operator, &value) {
                                Ok(true) => Some(Ok::<_, SegmentEvaluationError>(())),
                                Ok(false) => None,
                                Err(e) => Some(Err((e, segment, rule, &value).into())),
                            },
                        }
                    })
                    .transpose()?;
                // check if the candidate is good, or if the operator failed:
                candidate.is_some()
            }
        };
        // All rules must match:
        if !rule_result {
            return Ok(false);
        }
    }
    Ok(true)
}

fn check_operator(
    attribute_value: &Value,
    operator: &str,
    reference_value: &Value,
) -> std::result::Result<bool, CheckOperatorErrorDetail> {
    match operator {
        "is" => {
            if std::mem::discriminant(attribute_value) != std::mem::discriminant(reference_value) {
                Err(CheckOperatorErrorDetail::OperandsHaveDifferentTypes)
            } else {
                Ok(attribute_value == reference_value)
            }
        }
        "contains" => match (attribute_value, reference_value) {
            (Value::String(data), Value::String(reference_data)) => {
                Ok(data.contains(reference_data))
            }
            _ => Err(CheckOperatorErrorDetail::StringExpected),
        },
        "startsWith" => match (attribute_value, reference_value) {
            (Value::String(data), Value::String(reference_data)) => {
                Ok(data.starts_with(reference_data))
            }
            _ => Err(CheckOperatorErrorDetail::StringExpected),
        },
        "endsWith" => match (attribute_value, reference_value) {
            (Value::String(data), Value::String(reference_data)) => {
                Ok(data.ends_with(reference_data))
            }
            _ => Err(CheckOperatorErrorDetail::StringExpected),
        },
        "greaterThan" => match (attribute_value, reference_value) {
            // TODO: Go implementation also compares strings (by parsing them as floats). Do we need this?
            //       https://github.com/IBM/appconfiguration-go-sdk/blob/master/lib/internal/models/Rule.go#L82
            // TODO: we could have numbers not representable as f64, maybe we should try to parse it to i64 and u64 too?
            (Value::Numeric(data), Value::Numeric(reference_data)) => Ok(data > reference_data),
            _ => Err(CheckOperatorErrorDetail::EntityAttrNotANumber),
        },
        "lesserThan" => match (attribute_value, reference_value) {
            (Value::Numeric(data), Value::Numeric(reference_data)) => Ok(data < reference_data),
            _ => Err(CheckOperatorErrorDetail::EntityAttrNotANumber),
        },
        "greaterThanEquals" => match (attribute_value, reference_value) {
            (Value::Numeric(data), Value::Numeric(reference_data)) => Ok(data >= reference_data),
            _ => Err(CheckOperatorErrorDetail::EntityAttrNotANumber),
        },
        "lesserThanEquals" => match (attribute_value, reference_value) {
            (Value::Numeric(data), Value::Numeric(reference_data)) => Ok(data <= reference_data),
            _ => Err(CheckOperatorErrorDetail::EntityAttrNotANumber),
        },
        _ => Err(CheckOperatorErrorDetail::OperatorNotImplemented),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::errors::{EntityEvaluationError, Error};
    use crate::models::{ConfigValue, Segment, SegmentRule, Segments, TargetingRule};
    use rstest::*;

    #[fixture]
    fn segments() -> HashMap<String, Segment> {
        HashMap::from([(
            "some_segment_id_1".into(),
            Segment {
                name: "".into(),
                segment_id: "some_segment_id_1".into(),
                description: "".into(),
                tags: None,
                rules: vec![SegmentRule {
                    attribute_name: "name".into(),
                    operator: "is".into(),
                    values: vec![ConfigValue(serde_json::json!("heinz"))],
                }],
            },
        )])
    }

    #[fixture]
    fn segment_rules() -> Vec<TargetingRule> {
        vec![TargetingRule {
            rules: vec![Segments {
                segments: vec!["some_segment_id_1".into()],
            }],
            value: ConfigValue(serde_json::Value::Number((-48).into())),
            order: 0,
            rollout_percentage: Some(ConfigValue(serde_json::Value::Number((100).into()))),
        }]
    }

    // SCENARIO - If the SDK user fail to pass the “attributes” for evaluation of featureflag which is segmented - we have considered that evaluation as “does not belong to any segment” and we serve the enabled_value.
    // EXAMPLE - Assume two teams are using same featureflag. One team is interested only in enabled_value & disabled_value. This team doesn’t pass attributes for  their evaluation. Other team wants to have overridden_value, as a result they update the featureflag by adding segment rules to it. This team passes attributes in their evaluation to get the overridden_value for matching segment, and enabled_value for non-matching segment.
    //  We should not fail the evaluation.
    #[rstest]
    fn test_attribute_not_found(
        segments: HashMap<String, Segment>,
        segment_rules: Vec<TargetingRule>,
    ) {
        let entity = crate::tests::GenericEntity {
            id: "a2".into(),
            attributes: HashMap::from([("name2".into(), Value::from("heinz".to_string()))]),
        };
        let rule =
            find_applicable_segment_rule_for_entity(&segments, segment_rules.into_iter(), &entity);
        // Segment evaluation should not fail:
        let rule = rule.unwrap();
        // But no segment should be found:
        assert!(rule.is_none())
    }

    // SCENARIO - The segment_id present in featureflag is invalid. In other words - the /config json dump has a featureflag, which has segment_rules. The segment_id in this segment_rules is invalid. Because this segment_id is not found in segments array.
    // This is a very good question. Firstly, the our server-side API are strongly validating inputs and give the responses. We have unittests & integration tests that verifies the input & output of /config API.  The response is always right. It is very much rare scenario where the API response has segment_id in featureflag object, that is not present is segments array.
    // We can agree to return error and mark evaluation as failed.
    #[rstest]
    fn test_invalid_segment_id(segments: HashMap<String, Segment>) {
        let entity = crate::tests::GenericEntity {
            id: "a2".into(),
            attributes: HashMap::from([("name".into(), Value::try_from(42.0).unwrap())]),
        };
        let segment_rules = vec![TargetingRule {
            rules: vec![Segments {
                segments: vec!["non_existing_segment_id".into()],
            }],
            value: ConfigValue(serde_json::Value::Number((-48).into())),
            order: 0,
            rollout_percentage: Some(ConfigValue(serde_json::Value::Number((100).into()))),
        }];
        let rule =
            find_applicable_segment_rule_for_entity(&segments, segment_rules.into_iter(), &entity);
        // Error message should look something like this:
        //  Failed to evaluate entity: Failed to evaluate entity 'a2' against targeting rule '0'.
        //  Caused by: Segment 'non_existing_segment_id' not found.
        // We are checking here that the parts are present to allow debugging of config by the user:
        let e = rule.unwrap_err();
        assert!(matches!(e, Error::EntityEvaluationError(_)));
        let Error::EntityEvaluationError(EntityEvaluationError(
            SegmentEvaluationError::SegmentIdNotFound(ref segment_id),
        )) = e
        else {
            panic!("Error type mismatch!");
        };
        assert_eq!(segment_id, "non_existing_segment_id");
    }

    // SCENARIO - evaluating an operator fails. Meaning, [for example] user has added a numeric value(int/float) in appconfig segment attribute, but in their application they pass the attribute with a boolean value.
    // We can mark this as failure and return error.
    #[rstest]
    fn test_operator_failed(segments: HashMap<String, Segment>, segment_rules: Vec<TargetingRule>) {
        let entity = crate::tests::GenericEntity {
            id: "a2".into(),
            attributes: HashMap::from([("name".into(), Value::try_from(42.0).unwrap())]),
        };
        let rule =
            find_applicable_segment_rule_for_entity(&segments, segment_rules.into_iter(), &entity);
        let e = rule.unwrap_err();
        assert!(matches!(e, Error::EntityEvaluationError(_)));
        let Error::EntityEvaluationError(EntityEvaluationError(
            SegmentEvaluationError::SegmentEvaluationFailed(ref error),
        )) = e
        else {
            panic!("Error type mismatch!");
        };
        assert_eq!(error.segment_id, "some_segment_id_1");
        assert_eq!(error.segment_rule.attribute_name, "name");
        assert_eq!(error.value, Value::String("heinz".into()));
    }
}
