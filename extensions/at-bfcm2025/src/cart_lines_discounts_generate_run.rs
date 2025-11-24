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

// Eligible SKUs for Artist & Trade BFCM 2025 discount (matches BFCM 2025 logic)
const ELIGIBLE_SKUS: &[&str] = &[
    "10-088", "10-122", "10-123", "10-124", "10-125", "10-129", "10-164", "10-175",
    "10-182", "10-208", "10-260", "10-261", "21-067", "21-068", "21-069", "21-070",
    "22-004", "22-005", "23-001", "23-002", "23-003", "23-004", "23-005", "23-006",
    "23-007", "23-008", "23-009", "23-010", "23-011", "23-012", "23-013", "23-014",
    "23-015", "23-016", "23-017", "23-018", "23-019", "23-020", "23-021", "23-022",
    "23-023", "23-024", "23-025", "23-026", "23-027", "23-028", "23-029", "23-030",
    "23-031", "23-032", "23-033", "23-034", "23-035", "23-036", "23-037", "23-038",
    "23-039", "23-040", "23-041", "23-042", "23-043", "23-044", "23-045", "23-046",
    "23-047", "23-048", "23-049", "23-050", "23-051", "23-052", "23-053", "23-054",
    "23-055", "23-056", "23-057", "23-058", "23-059", "23-060", "23-061", "23-062",
    "23-063", "23-064", "23-065", "23-066", "23-067", "23-068", "23-069", "23-070",
    "23-071", "23-072", "23-073", "23-074", "23-075", "23-076", "23-077", "23-078",
    "23-079", "23-080", "23-081", "23-082", "23-083", "23-084", "23-085", "23-086",
    "23-087", "23-088", "23-089", "23-090", "23-091", "23-092", "23-093", "23-094",
    "23-095", "23-096", "23-097", "23-098", "23-099", "23-100", "23-101", "23-102",
    "23-103", "23-104", "23-105", "23-106", "23-107", "23-108", "23-109", "23-110",
    "23-111", "23-112", "23-113", "CUSTOM-FRAME-DIGITAL", "CUSTOM-FRAME-MG-DIGITAL",
    "CUSTOM-FRAME-XG-DIGITAL",
];

#[shopify_function]
fn cart_lines_discounts_generate_run(
    input: schema::cart_lines_discounts_generate_run::Input,
) -> Result<CartLinesDiscountsGenerateRunResult> {
    // Only proceed for product class discounts
    let has_product_discount_class = input
        .discount()
        .discount_classes()
        .contains(&DiscountClass::Product);

    if !has_product_discount_class {
        return Ok(CartLinesDiscountsGenerateRunResult { operations: vec![] });
    }

    let cart_lines = input.cart().lines();
    let eligible_skus: HashSet<&str> = ELIGIBLE_SKUS.iter().copied().collect();

    // First pass: collect all frame spec numbers from eligible parent SKUs
    let eligible_frame_specs: HashSet<String> = cart_lines
        .iter()
        .filter_map(|line| {
            use schema::cart_lines_discounts_generate_run::input::cart::lines::Merchandise;
            let sku = match line.merchandise() {
                Merchandise::ProductVariant(variant) => variant.sku()?,
                _ => return None,
            };

            if eligible_skus.contains(sku.as_str()) && sku != "CUSTOM-FRAME-ADDITION" {
                line.frame_spec_number()
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
        use schema::cart_lines_discounts_generate_run::input::cart::lines::Merchandise;
        let sku = match line.merchandise() {
            Merchandise::ProductVariant(variant) => match variant.sku() {
                Some(s) => s,
                None => continue,
            },
            _ => continue,
        };

        let should_discount = if sku == "CUSTOM-FRAME-ADDITION" {
            line.parent_design()
                .and_then(|attr| attr.value())
                .map(|parent_design| eligible_frame_specs.contains(parent_design))
                .unwrap_or(false)
        } else {
            eligible_skus.contains(sku.as_str())
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

    if discount_targets.is_empty() {
        return Ok(CartLinesDiscountsGenerateRunResult { operations: vec![] });
    }

    let operations = vec![CartOperation::ProductDiscountsAdd(
        ProductDiscountsAddOperation {
            selection_strategy: ProductDiscountSelectionStrategy::First,
            candidates: vec![ProductDiscountCandidate {
                targets: discount_targets,
                message: Some("Additional 5% Off Digital Photo Frames".to_string()),
                value: ProductDiscountCandidateValue::Percentage(Percentage {
                    value: Decimal(5.0),
                }),
                associated_discount_code: Some(schema::AssociatedDiscountCode {
                    code: "ARTISTTRADE5".to_string(),
                }),
            }],
        },
    )];

    Ok(CartLinesDiscountsGenerateRunResult { operations })
}
