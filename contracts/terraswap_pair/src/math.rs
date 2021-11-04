use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::Uint128;
use fixed::transcendental::pow as fixed_pow;
use fixed::types::I64F64;
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

pub type FixedFloat = I64F64;

pub fn calc_out_given_in(
    balance_in: Uint128,
    weight_in: FixedFloat,
    balance_out: Uint128,
    weight_out: FixedFloat,
    amount_in: Uint128,
) -> Uint128 {
    if amount_in.is_zero() {
        return Uint128::zero();
    }

    // Use 256 so that to prevent overflow error
    let balance_in: Uint256 = balance_in.into();
    let adjusted_in = balance_in.add(amount_in.into());

    let y256 = Decimal256::from_ratio(balance_in, adjusted_in).to_string();
    let y = FixedFloat::from_str(&y256).unwrap();

    let weight_ratio = weight_in.div(&weight_out);

    let multiplier: FixedFloat = fixed_pow(y, weight_ratio).unwrap();
    let multiplier = FixedFloat::from_num(1).sub(multiplier);

    let amount_out: u128 = FixedFloat::from_num(balance_out.u128())
        .mul(&multiplier)
        .to_num();

    Uint128::from(amount_out)
}

pub fn calc_in_given_out(
    balance_in: Uint128,
    weight_in: FixedFloat,
    balance_out: Uint128,
    weight_out: FixedFloat,
    amount_out: Uint128,
) -> Uint128 {
    // Use 256 so that to prevent overflow error
    let balance_out: Uint256 = balance_out.into();
    let updated_balance = balance_out.sub(amount_out.into());

    let weight_ratio = weight_out.div(&weight_in);

    let y256 = Decimal256::from_ratio(balance_out, updated_balance).to_string();
    let y = FixedFloat::from_str(&y256).unwrap();

    let multiplier: FixedFloat = fixed_pow(y, weight_ratio).unwrap();
    let multiplier = multiplier.sub(FixedFloat::from_num(1));

    let amount_in: u128 = FixedFloat::from_num(balance_in.u128())
        .mul(&multiplier)
        .to_num();

    Uint128::from(amount_in)
}
