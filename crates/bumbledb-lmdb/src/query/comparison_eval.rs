use super::*;

pub(super) fn comparisons_ready_pass(
    txn: &ReadTxn<'_>,
    comparisons: &[NormPredicate],
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left_encoded) = operand_encoded_value(
            &comparison.operands[0],
            &comparison.value_type,
            inputs,
            binding,
        ) else {
            continue;
        };
        let Some(right_encoded) = operand_encoded_value(
            &comparison.operands[1],
            &comparison.value_type,
            inputs,
            binding,
        ) else {
            continue;
        };
        if encoded_comparison_supported(comparison.op, &comparison.value_type) {
            counters.comparisons_evaluated += 1;
            counters.encoded_comparisons_evaluated += 1;
            if !compare_encoded_values(
                left_encoded.as_bytes(),
                comparison.op,
                right_encoded.as_bytes(),
            ) {
                counters.comparisons_failed += 1;
                return Ok(false);
            }
            continue;
        }

        let Some(left) = operand_logical_value(
            txn,
            &comparison.operands[0],
            &comparison.value_type,
            query,
            inputs,
            binding,
            counters,
        )?
        else {
            continue;
        };
        let Some(right) = operand_logical_value(
            txn,
            &comparison.operands[1],
            &comparison.value_type,
            query,
            inputs,
            binding,
            counters,
        )?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        counters.decoded_comparisons_evaluated += 1;
        if !compare_values(&left, comparison.op, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn operand_encoded_value(
    operand: &NormOperand,
    _value_type: &ValueType,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
) -> Option<EncodedOwned> {
    match operand {
        NormOperand::Var(variable) => binding.get(variable.0 as usize).cloned(),
        NormOperand::Input(input) => inputs.get(*input).cloned(),
        NormOperand::Literal(literal) => Some(literal.clone()),
    }
}

fn encoded_comparison_supported(operator: ComparisonOperator, value_type: &ValueType) -> bool {
    match operator {
        ComparisonOperator::Eq | ComparisonOperator::NotEq => true,
        ComparisonOperator::Lt
        | ComparisonOperator::Lte
        | ComparisonOperator::Gt
        | ComparisonOperator::Gte => !matches!(value_type, ValueType::String | ValueType::Bytes),
    }
}

fn compare_encoded_values(left: &[u8], operator: ComparisonOperator, right: &[u8]) -> bool {
    match operator {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
}
