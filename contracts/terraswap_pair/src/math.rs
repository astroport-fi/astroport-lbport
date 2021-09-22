use cosmwasm_std::Uint128;
use fixed::transcendental::pow as fixed_pow;
use fixed::types::I64F64;
use std::ops::{Add, Div, Mul, Sub};
use terraswap::asset::WeightedAsset;
use terraswap::U256;

pub type FixedFloat = I64F64;

/////////////////////////////////////////////////////////////
pub const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128);

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

    let y =
        FixedFloat::from_num(balance_in.u128() * DECIMAL_FRACTIONAL.u128() / adjusted_in.u128());
    let y = y.div(&FixedFloat::from_num(DECIMAL_FRACTIONAL.u128()));

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

    let y = FixedFloat::from_num(
        balance_out.u128() * DECIMAL_FRACTIONAL.u128() / updated_balance.u128(),
    );
    let y = y.div(&FixedFloat::from_num(DECIMAL_FRACTIONAL.u128()));

    let multiplier: FixedFloat = fixed_pow(y, weight_ratio).unwrap();
    let multiplier = multiplier.sub(FixedFloat::from_num(1));

    let amount_in: u128 = FixedFloat::from_num(balance_in.u128())
        .mul(&multiplier)
        .to_num();

    Uint128::from(amount_in)
}

pub fn calc_share(
    total_share: Uint128,
    deposits: [Uint128; 2],
    pools: [WeightedAsset; 2],
) -> Uint128 {
    if total_share.is_zero() {
        // Initial share = collateral amount
        Uint128::new(
            (U256::from(deposits[0].u128()) * U256::from(deposits[1].u128()))
                .integer_sqrt()
                .as_u128(),
        )
    } else {
        // min(1, 2)
        // 1. sqrt(deposit_0 * exchange_rate_0_to_1 * deposit_0) * (total_share / sqrt(pool_0 * pool_1))
        // == deposit_0 * total_share / pool_0
        // 2. sqrt(deposit_1 * exchange_rate_1_to_0 * deposit_1) * (total_share / sqrt(pool_1 * pool_1))
        // == deposit_1 * total_share / pool_1
        std::cmp::min(
            deposits[0].multiply_ratio(total_share, pools[0].amount),
            deposits[1].multiply_ratio(total_share, pools[1].amount),
        )
    }
}

#[cfg(test)]
mod unit {
    use super::*;
    use terraswap::asset::AssetInfo;

    fn default_weighted_asset() -> WeightedAsset {
        WeightedAsset {
            info: AssetInfo::NativeToken {
                denom: "foo".to_string(),
            },
            amount: Uint128::from(0u128),
            start_weight: Uint128::from(1u128),
            end_weight: Uint128::from(1u128),
        }
    }

    #[test]
    fn test_overflow_calc_share() {
        let actual = calc_share(
            Uint128::from(0u128),
            [Uint128::MAX, Uint128::MAX],
            [default_weighted_asset(), default_weighted_asset()],
        );
        assert_eq!(u128::MAX, actual.u128())
    }
}
