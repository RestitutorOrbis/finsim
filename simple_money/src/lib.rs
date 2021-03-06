use std::collections::HashMap;
use std::ops::{Add, Sub, Mul};
use std::cmp::Ordering;
use rust_decimal::Decimal;
use thiserror::Error;
use rust_decimal_macros::*;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Currency {
    CAD,
    USD,
}

#[derive(Debug,Error)]
pub enum MoneyError{
    #[error("Could not find exchange rate")]
    CouldNotFindExchangeRate,
    #[error("Mismatched currencies")]
    MismatchedCurrencies,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Money {
    pub amount: Decimal,
    pub currency: Currency,
}

#[derive(PartialEq, Eq, Hash)]
struct ExchangeRateQuery{
    from: Currency,
    to: Currency,
}

pub struct Exchange {
    rates: HashMap<ExchangeRateQuery, Decimal>,
}

impl Exchange {
    pub fn new() -> Exchange {
        return Exchange{
            rates: HashMap::new(),
        } 
    }

    pub fn set_rate(&mut self, from: Currency, to: Currency, rate: Decimal){
        let key = ExchangeRateQuery { from, to };
        let inverse_key = ExchangeRateQuery{ from: to, to: from };
        self.rates.insert(key, rate);
        self.rates.insert(inverse_key, Decimal::new(1, 0) / rate);
    }

    pub fn get_rate(&self, from: Currency, to: Currency) -> Result<Decimal, MoneyError>{
        let key = ExchangeRateQuery { from, to };
        let rate = self.rates.get(&key);
        if let Some(rate) = rate {
            Ok(*rate)
        }else{
            Err(MoneyError::CouldNotFindExchangeRate)
        }
    }

    pub fn convert(&self, money: Money, currency: Currency) -> Result<Money, MoneyError> {
        if money.currency == currency {
            return Ok(money);
        }

        let rate = self.get_rate(money.currency, currency)?;
        let converted_money = Money { amount: money.amount * rate, currency: currency };
        return Ok(converted_money);
    }
    
    pub fn add(&self, first: Money, second: Money, output_currency: Currency) -> Result<Money, MoneyError> {
        if first.currency == output_currency && second.currency == output_currency {
            Ok(first + second)
        }else{
            let first_in_output_currency = self.convert(first, output_currency)?;
            let second_in_output_currency =self.convert(second, output_currency)?;
            Ok(first_in_output_currency + second_in_output_currency)
        }
    }

    pub fn sub(&self, first: Money, second: Money, output_currency: Currency) -> Result<Money, MoneyError> {
        if first.currency == output_currency && second.currency == output_currency {
            Ok(first - second)
        }else{
            let first_in_output_currency = self.convert(first, output_currency)?;
            let second_in_output_currency =self.convert(second, output_currency)?;
            Ok(first_in_output_currency - second_in_output_currency)
        }
    }

    pub fn lt(&self, first: Money, second: Money) -> Result<bool, MoneyError> {
        if first.currency == second.currency {
            Ok(first < second)
        }else{
            let second_in_first_currency = self.convert(second, first.currency)?;
            Ok(first < second_in_first_currency)
        }
    }

    pub fn lte(&self, first: Money, second: Money) -> Result<bool, MoneyError> {
        if first.currency == second.currency {
            Ok(first <= second)
        }else{
            let second_in_first_currency = self.convert(second, first.currency)?;
            Ok(first <= second_in_first_currency)
        }
    }
    
    pub fn eq(&self, first: Money, second: Money) -> Result<bool, MoneyError> {
        if first.currency == second.currency {
            Ok(first == second)
        }else{
            let second_in_first_currency = self.convert(second, first.currency)?;
            Ok(first == second_in_first_currency)
        }
    }

    pub fn gte(&self, first: Money, second: Money) -> Result<bool, MoneyError> {
        if first.currency == second.currency {
            Ok(first >= second)
        }else{
            let second_in_first_currency = self.convert(second, first.currency)?;
            Ok(first >= second_in_first_currency)
        }
    }

    pub fn gt(&self, first: Money, second: Money) -> Result<bool, MoneyError> {
        if first.currency == second.currency {
            Ok(first > second)
        }else{
            let second_in_first_currency = self.convert(second, first.currency)?;
            Ok(first > second_in_first_currency)
        }
    }

    pub fn clamp(&self, input: Money, min: Money, max: Money, output_currency: Currency) -> Result<Money, MoneyError>{
        if input.currency == output_currency && min.currency == output_currency && max.currency == output_currency {
            Ok(input.clamp(min, max))
        }else{
            let input_in_output_currency = self.convert(input, output_currency)?;
            let min_in_output_currency = self.convert(min, output_currency)?;
            let max_in_output_currency = self.convert(max, output_currency)?;

            Ok(input_in_output_currency.clamp(min_in_output_currency, max_in_output_currency))
        }
    }
}

impl PartialOrd for Money {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.currency != other.currency {
            return None;
        }

        return self.amount.partial_cmp(&other.amount)
    }
}

impl Ord for Money {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some(ordering) = self.partial_cmp(other) {
            ordering
        }else{
            panic!("Couldn't compare Money objects")
        }
    }
}

impl Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.currency != other.currency {
            panic!("Currency mismatch");
        }

        Self { amount: self.amount + other.amount, currency: self.currency }
    }
}

impl Sub for Money {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.currency != other.currency {
            panic!("Currency mismatch");
        }

        Self { amount: self.amount - other.amount, currency: self.currency }
    }
}

impl Mul<Decimal> for Money {
    type Output = Self;

     fn mul(self, rhs: Decimal) -> Self::Output {
        Self { amount: self.amount * rhs, currency: self.currency }
     }
}

pub trait RoundedEq{
    fn rounded_eq(&self, other: Self, dp: u32) -> bool;
}

impl RoundedEq for Money{
    fn rounded_eq(&self, other: Self, dp: u32) -> bool {
        if self.currency != other.currency {
            return false;
        }


        let rounded_self_amount = self.amount.round_dp(dp);
        let rounded_other_amount = other.amount.round_dp(dp);

        return rounded_self_amount == rounded_other_amount;
    }
}

#[macro_export]
macro_rules! cad_money {
    ($amount: expr) => {
        Money { amount: dec!($amount), currency: Currency::CAD }
    };
}

#[macro_export]
macro_rules! usd_money {
    ($amount: expr) => {
        Money { amount: dec!($amount), currency: Currency::USD }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_rounded_eq {
        ($lhs:expr, $rhs:expr) => {
            assert!(
                $lhs.rounded_eq($rhs, 2), 
                "assertion failed! left: {:?}, right: {:?}", 
                $lhs, 
                $rhs
            )
        };
    }

    fn setup() -> Exchange {
        let mut exchange = Exchange::new();
        let rate = Decimal::new(13, 1);
        exchange.set_rate(
            Currency::USD, 
            Currency::CAD, 
            rate,
        );
        return exchange;
    }

    #[test]
    fn can_compare_same_currencies(){
        let one = usd_money!(1);
        let two = usd_money!(2);

        assert_eq!(one < two, true);
        assert_eq!(two > one, true);
        assert_eq!(two == one, false);
        assert_eq!(two == two, true);
    }

    #[test]
    fn can_compare_usd_amount_with_greater_cad_amount(){
        let exchange = setup();

        let usd_money = usd_money!(1);
        let cad_money = cad_money!(2);

        assert_eq!(exchange.lt(usd_money, cad_money).unwrap(), true);
        assert_eq!(exchange.lte(usd_money, cad_money).unwrap(), true);
        assert_eq!(exchange.eq(usd_money, cad_money).unwrap(), false);
        assert_eq!(exchange.gte(usd_money, cad_money).unwrap(), false);
        assert_eq!(exchange.gt(usd_money, cad_money).unwrap(), false);

        assert_eq!(exchange.lt(cad_money, usd_money).unwrap(), false);
        assert_eq!(exchange.lte(cad_money, usd_money).unwrap(), false);
        assert_eq!(exchange.eq(cad_money, usd_money).unwrap(), false);
        assert_eq!(exchange.gte(cad_money, usd_money).unwrap(), true);
        assert_eq!(exchange.gt(cad_money, usd_money).unwrap(), true);
    }

    #[test]
    fn can_compare_usd_amount_with_less_cad_amount(){
        let exchange = setup();

        let usd_money = usd_money!(2);
        let cad_money = cad_money!(1);

        assert_eq!(exchange.lt(usd_money, cad_money).unwrap(), false);
        assert_eq!(exchange.lte(usd_money, cad_money).unwrap(), false);
        assert_eq!(exchange.eq(usd_money , cad_money).unwrap(), false);
        assert_eq!(exchange.gte(usd_money, cad_money).unwrap(), true);
        assert_eq!(exchange.gt(usd_money, cad_money).unwrap(), true);

        assert_eq!(exchange.lt(cad_money, usd_money).unwrap(), true);
        assert_eq!(exchange.lte(cad_money, usd_money).unwrap(), true);
        assert_eq!(exchange.eq(cad_money , usd_money).unwrap(), false);
        assert_eq!(exchange.gte(cad_money, usd_money).unwrap(), false);
        assert_eq!(exchange.gt(cad_money, usd_money).unwrap(), false);
    }

    #[test]
    fn can_compare_usd_amount_with_equal_cad_amount(){
        let exchange = setup();

        let usd_money = usd_money!(1);
        let cad_money = cad_money!(1.3);

        assert_eq!(exchange.lt(usd_money, cad_money).unwrap(), false);
        assert_eq!(exchange.lte(usd_money, cad_money).unwrap(), true);
        assert_eq!(exchange.eq(usd_money , cad_money).unwrap(), true);
        assert_eq!(exchange.gte(usd_money, cad_money).unwrap(), true);
        assert_eq!(exchange.gt(usd_money, cad_money).unwrap(), false);

        assert_eq!(exchange.lt(cad_money, usd_money).unwrap(), false);
        assert_eq!(exchange.lte(cad_money, usd_money).unwrap(), true);
        assert_eq!(exchange.eq(cad_money , usd_money).unwrap(), true);
        assert_eq!(exchange.gte(cad_money, usd_money).unwrap(), true);
        assert_eq!(exchange.gt(cad_money, usd_money).unwrap(), false);
    }

    #[test]
    fn can_clamp_with_value_less_than_range(){
        let exchange = setup();        

        let input = usd_money!(1);
        let min = cad_money!(2);
        let max = cad_money!(3);

        let clamped_cad = exchange.clamp(input, min, max, Currency::CAD).unwrap();
        let clamped_usd = exchange.clamp(input, min, max, Currency::USD).unwrap();

        let expected_clamped_cad = cad_money!(2);
        let expected_clamped_usd_amount = dec!(2) * dec!(1)/dec!(1.3);
        let expected_clamped_usd = Money { amount: expected_clamped_usd_amount, currency: Currency::USD };

        assert_rounded_eq!(clamped_cad, expected_clamped_cad);
        assert_rounded_eq!(clamped_usd, expected_clamped_usd);
    }

    #[test]
    fn can_clamp_with_value_within_range(){
        let exchange = setup();

        let input = cad_money!(2.5);
        let min = cad_money!(2);
        let max = cad_money!(3);

        let clamped_cad = exchange.clamp(input, min, max, Currency::CAD).unwrap();
        let clamped_usd = exchange.clamp(input, min, max, Currency::USD).unwrap();

        let expected_clamped_cad = cad_money!(2.5);
        let expected_usd_amount = dec!(2.5) * (dec!(1)/dec!(1.3));
        let expected_clamped_usd = Money { amount: expected_usd_amount, currency: Currency::USD };
        
        assert_rounded_eq!(clamped_cad, expected_clamped_cad);
        assert_rounded_eq!(clamped_usd, expected_clamped_usd);
    }

    #[test]
    fn can_clamp_with_value_greater_than_range(){
        let exchange = setup();

        let input = cad_money!(2);
        let min = usd_money!(0);
        let max = usd_money!(0.5);

        let clamped_cad = exchange.clamp(input, min, max, Currency::CAD).unwrap();
        let clamped_usd = exchange.clamp(input, min, max, Currency::USD).unwrap();

        let expected_clamped_cad = cad_money!(0.65);
        let expected_clamped_usd = usd_money!(0.5);

        assert_rounded_eq!(clamped_cad, expected_clamped_cad);
        assert_rounded_eq!(clamped_usd, expected_clamped_usd);
    }

    #[test]
    fn can_add_different_currencies_and_get_converted_result(){
        let exchange = setup();

        let first = cad_money!(1); 
        let second = usd_money!(1);
        
        let sum_in_cad = exchange.add(first, second, Currency::CAD).unwrap();
        let sum_in_usd = exchange.add(first, second, Currency::USD).unwrap();

        let expected_cad_sum = cad_money!(2.3);
        let expected_usd_sum_amount = dec!(1) * dec!(1)/dec!(1.3) + dec!(1);
        let expected_usd_sum = Money { amount: expected_usd_sum_amount, currency: Currency::USD };

        assert_rounded_eq!(sum_in_cad, expected_cad_sum);
        assert_rounded_eq!(sum_in_usd, expected_usd_sum);
    }

    #[test]
    fn can_add_same_currencies_and_get_converted_result(){
        let exchange = setup();

        let first = cad_money!(1);
        let second = cad_money!(1);

        let sum_in_cad = exchange.add(first, second, Currency::CAD).unwrap();
        let sum_in_usd = exchange.add(first, second, Currency::USD).unwrap();

        let expected_cad_sum = cad_money!(2);
        let expected_usd_sum_amount = dec!(1) * dec!(1)/dec!(1.3) * dec!(2);
        let expected_usd_sum = Money { amount: expected_usd_sum_amount, currency: Currency::USD };

        assert_rounded_eq!(sum_in_cad, expected_cad_sum);
        assert_rounded_eq!(sum_in_usd, expected_usd_sum);
    }

    #[test]
    fn can_subtract_different_currencies_and_get_converted_result(){
        let exchange = setup();

        let first = cad_money!(2);
        let second = usd_money!(1);

        let diff_in_cad = exchange.sub(first, second, Currency::CAD).unwrap();
        let diff_in_usd = exchange.sub(first, second, Currency::USD).unwrap();

        let expected_cad_diff = cad_money!(0.7);
        let expected_usd_diff_amount = dec!(2) * dec!(1)/dec!(1.3) - dec!(1);
        let expected_usd_diff = Money { amount: expected_usd_diff_amount, currency: Currency::USD };

        assert_rounded_eq!(diff_in_cad, expected_cad_diff);
        assert_rounded_eq!(diff_in_usd, expected_usd_diff);
    }

    #[test]
    fn can_subtract_same_currencies_and_get_converted_result(){
        let exchange = setup();

        let first = cad_money!(2);
        let second = cad_money!(1);

        let diff_in_cad = exchange.sub(first, second, Currency::CAD).unwrap();
        let diff_in_usd = exchange.sub(first, second, Currency::USD).unwrap();

        let expected_cad_diff = cad_money!(1);
        let expected_usd_diff_amount = dec!(1) * dec!(1)/dec!(1.3);
        let expected_usd_diff = Money { amount: expected_usd_diff_amount, currency: Currency::USD };

        assert_rounded_eq!(diff_in_cad, expected_cad_diff);
        assert_rounded_eq!(diff_in_usd, expected_usd_diff);
    }
}