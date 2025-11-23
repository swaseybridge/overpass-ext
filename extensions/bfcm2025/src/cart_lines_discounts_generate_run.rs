use crate::schema::CartLineTarget;
use crate::schema::CartLinesDiscountsGenerateRunResult;
use crate::schema::CartOperation;
use crate::schema::DiscountClass;
use crate::schema::Percentage;
use crate::schema::ProductDiscountCandidate;
use crate::schema::ProductDiscountCandidateTarget;
use crate::schema::ProductDiscountCandidateValue;
use crate::schema::ProductDiscountSelectionStrategy;
use crate::schema::ProductDiscountsAddOperation;

use super::schema;
use shopify_function::prelude::*;
use shopify_function::Result;
use std::collections::HashSet;

#[shopify_function]
fn cart_lines_discounts_generate_run(
    input: schema::cart_lines_discounts_generate_run::Input,
) -> Result<CartLinesDiscountsGenerateRunResult> {
    // Check if the discount has the PRODUCT class
    let has_product_discount_class = input
        .discount()
        .discount_classes()
        .contains(&DiscountClass::Product);

    if !has_product_discount_class {
        return Ok(CartLinesDiscountsGenerateRunResult { operations: vec![] });
    }

    let cart_lines = input.cart().lines();

    // First pass: collect all frame spec numbers from CUSTOM-FRAME-DIGITAL lines
    let digital_frame_specs: HashSet<String> = cart_lines
        .iter()
        .filter_map(|line| {
            // Check if this is a CUSTOM-FRAME-DIGITAL line
            let sku = line.merchandise()?.sku()?;
            if sku == "CUSTOM-FRAME-DIGITAL" {
                // Find the _frame_spec_number attribute
                line.attributes()
                    .iter()
                    .find(|attr| attr.key() == "_frame_spec_number")
                    .and_then(|attr| attr.value())
                    .map(|v| v.to_string())
            } else {
                None
            }
        })
        .collect();

    // Second pass: collect all eligible line IDs for discount
    let mut discount_targets = vec![];

    for line in cart_lines.iter() {
        let sku = match line.merchandise().and_then(|m| m.sku()) {
            Some(s) => s,
            None => continue,
        };

        let should_discount = match sku.as_str() {
            "CUSTOM-FRAME-DIGITAL" => {
                // Always discount digital frames
                true
            }
            "CUSTOM-FRAME-ADDITION" => {
                // Only discount if parent is a digital frame
                line.attributes()
                    .iter()
                    .find(|attr| attr.key() == "_parent_design")
                    .and_then(|attr| attr.value())
                    .map(|parent_design| digital_frame_specs.contains(parent_design))
                    .unwrap_or(false)
            }
            _ => false,
        };

        if should_discount {
            discount_targets.push(ProductDiscountCandidateTarget::CartLine(
                CartLineTarget {
                    id: line.id().clone(),
                    quantity: None,
                },
            ));
        }
    }

    // If no eligible lines, return empty operations
    if discount_targets.is_empty() {
        return Ok(CartLinesDiscountsGenerateRunResult { operations: vec![] });
    }

    // Create the discount operation
    let operations = vec![CartOperation::ProductDiscountsAdd(
        ProductDiscountsAddOperation {
            selection_strategy: ProductDiscountSelectionStrategy::First,
            candidates: vec![ProductDiscountCandidate {
                targets: discount_targets,
                message: Some("15% off digital frames".to_string()),
                value: ProductDiscountCandidateValue::Percentage(Percentage {
                    value: Decimal(15.0),
                }),
                associated_discount_code: None,
            }],
        },
    )];

    Ok(CartLinesDiscountsGenerateRunResult { operations })
}
