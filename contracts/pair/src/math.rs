use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::Uint128;
use fixed::transcendental::pow;
use fixed::types::I64F64 as FixedFloat;
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

pub fn calc_out_given_in(
    balance_in: Uint128,
    weight_in: Decimal256,
    balance_out: Uint128,
    weight_out: Decimal256,
    amount_in: Uint128,
) -> Uint128 {
    if amount_in.is_zero() {
        return Uint128::zero();
    }

    let adjusted_in = balance_in.add(amount_in);
    let y = decimal_from_ratio(balance_in, adjusted_in);

    let weight_ratio = weight_in.div(weight_out);

    let multiplier = FixedFloat::from_num(1).sub(fixed_pow(y, weight_ratio));

    let amount_out: u128 = FixedFloat::from_num(balance_out.u128())
        .mul(&multiplier)
        .to_num();

    Uint128::from(amount_out)
}

pub fn calc_in_given_out(
    balance_in: Uint128,
    weight_in: Decimal256,
    balance_out: Uint128,
    weight_out: Decimal256,
    amount_out: Uint128,
) -> Uint128 {
    let updated_balance = balance_out.checked_sub(amount_out).unwrap();
    let weight_ratio = weight_out.div(weight_in);

    let y = decimal_from_ratio(balance_out, updated_balance);

    let multiplier = fixed_pow(y, weight_ratio).sub(FixedFloat::from_num(1));

    let amount_in: u128 = FixedFloat::from_num(balance_in.u128())
        .mul(&multiplier)
        .to_num();

    Uint128::from(amount_in)
}

fn decimal_from_ratio(nom: Uint128, denom: Uint128) -> Decimal256 {
    // Use 256 to prevent overflow error
    let nom: Uint256 = nom.into();
    let denom: Uint256 = denom.into();

    Decimal256::from_ratio(nom, denom)
}

fn fixed_pow(n: Decimal256, i: Decimal256) -> FixedFloat {
    // Truncate Decimal256
    let n: FixedFloat = FixedFloat::from_str(&n.to_string()).unwrap();
    let i: FixedFloat = FixedFloat::from_str(&i.to_string()).unwrap();
    let p: FixedFloat = pow(n, i).unwrap();

    p
}

pub fn uint2dec(i: Uint128) -> Decimal256 {
    let i: Uint256 = i.into();
    Decimal256::from_uint256(i)
}
