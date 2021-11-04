use cosmwasm_std::Uint128;
use fixed::transcendental::pow as fixed_pow;
use fixed::types::I64F64;
use std::ops::{Add, Div, Mul, Sub};

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

    let adjusted_in = balance_in.add(amount_in);

    let y = FixedFloat::from_num(balance_in.u128()).div(&FixedFloat::from_num(adjusted_in.u128()));
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
    let updated_balance = balance_out.checked_sub(amount_out).unwrap();

    let weight_ratio = weight_out.div(&weight_in);

    let y =
        FixedFloat::from_num(balance_out.u128()).div(&FixedFloat::from_num(updated_balance.u128()));

    let multiplier: FixedFloat = fixed_pow(y, weight_ratio).unwrap();
    let multiplier = multiplier.sub(FixedFloat::from_num(1));

    let amount_in: u128 = FixedFloat::from_num(balance_in.u128())
        .mul(&multiplier)
        .to_num();

    Uint128::from(amount_in)
}
