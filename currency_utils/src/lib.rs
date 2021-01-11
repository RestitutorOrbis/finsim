use anyhow::Result;
use rusty_money::{FormattableCurrency, Exchange, Money, MoneyError, ExchangeRate};
use rust_decimal::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorCode {
    #[error("Could not find exchange rate")]
    CouldNotFindExchangeRate,
    #[error("Could not convert money to currency")]
    CouldNotConvert,
    #[error("Currencies did not match for conversion")]
    CouldNotMatchCurrencies,
    #[error("Amount of money was not sensible")]
    InvalidAmount,
    #[error("Ratio was invalid")]
    InvalidRatio,
}

impl From<MoneyError> for ErrorCode {
    fn from(money_error: MoneyError) -> Self {
        match money_error {
            MoneyError::InvalidAmount => ErrorCode::InvalidAmount,
            // Whenever we have an invalid currency, that's usually because an arithmetic operation on money involved multiple currencies
            MoneyError::InvalidCurrency => ErrorCode::CouldNotMatchCurrencies,
            MoneyError::InvalidRatio => ErrorCode::InvalidRatio,
        }
    }
}

fn convert<'a, T: FormattableCurrency>(exchange: &Exchange<'a, T>, money: &Money<'a, T>, currency: &T) -> Result<Money<'a, T>, ErrorCode> {
    let exchange_rate_pair = exchange.get_rate(money.currency(), currency);

    if let Some(exchange_rate_pair) = exchange_rate_pair {
        let cur_money = exchange_rate_pair.convert(money.clone())?;
        Ok(cur_money)
    } else{
        Err(ErrorCode::CouldNotFindExchangeRate)
    }
}

pub trait CurrencyIndependentClamp<'a, T: FormattableCurrency> {
    fn clamp(&self, min_money: &Money<'a, T>, max_money: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<Money<'a, T>, ErrorCode>;
}

pub trait CurrencyIndependentComparison<'a, T: FormattableCurrency> {
    fn currency_independent_lt(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode>;
    fn currency_independent_lte(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode>;
    fn currency_independent_gt(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode>;
    fn currency_independent_gte(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode>;
    fn currency_independent_eq(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode>;
}

pub trait CurrencyIndependentAdd<'a, T: FormattableCurrency> {
    fn add(
        &self,
        other: &Money<'a, T>,
        output_currency: &T,
        exchange: &Exchange<'a, T>,
    ) -> Result<Money<'a, T>, ErrorCode>;
}

pub trait CurrencyIndependentSub<'a, T: FormattableCurrency> {
    fn sub(
        &self,
        other: &Money<'a, T>,
        output_currency: &T,
        exchange: &Exchange<'a, T>,
    ) -> Result<Money<'a, T>, ErrorCode>;
}

impl<'a, T: FormattableCurrency> CurrencyIndependentComparison<'a, T> for Money<'a, T> {
    fn currency_independent_lt(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode> {
        let cur_money = convert(exchange, self, other.currency())?;
        Ok(cur_money.amount() < other.amount())
    }

    fn currency_independent_lte(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode> {
        let cur_money = convert(exchange, self, other.currency())?;
        Ok(cur_money.amount() <= other.amount())
    }

    fn currency_independent_gt(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode> {
        let cur_money = convert(exchange, self, other.currency())?;
        Ok(cur_money.amount() > other.amount())
    }

    fn currency_independent_gte(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode> {
        let cur_money = convert(exchange, self, other.currency())?;
        Ok(cur_money.amount() >= other.amount())
    }

    fn currency_independent_eq(&self, other: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<bool, ErrorCode> {
        let cur_money = convert(exchange, self, other.currency())?;
        Ok(cur_money.amount() == other.amount())
    }
}

pub enum PositionRelativeToRange{
    BeforeRange,
    WithinRange,
    AfterRange,
}

fn determine_relative_position_of_money_relative_to_range<'a, T:FormattableCurrency>(
    money_to_consider: &Money<'a, T>, 
    min_money: &Money<'a, T>, 
    max_money: &Money<'a, T>, 
    exchange: &Exchange<'a, T>) 
    -> Result<PositionRelativeToRange, ErrorCode> 
{
    let less_than_min_result = money_to_consider.currency_independent_lt(min_money, exchange)?;
    let less_than_max_result = money_to_consider.currency_independent_lt(max_money, exchange)?;

    if less_than_min_result {
        Ok(PositionRelativeToRange::BeforeRange)
    }else if less_than_max_result {
        Ok(PositionRelativeToRange::WithinRange)
    }else{
        Ok(PositionRelativeToRange::AfterRange)
    }
}

impl<'a, T: FormattableCurrency> CurrencyIndependentClamp<'a, T> for Money<'a, T>{
    fn clamp(&self, min_money: &Money<'a, T>, max_money: &Money<'a, T>, exchange: &Exchange<'a, T>) -> Result<Money<'a, T>, ErrorCode> {
        let relative_to_range = determine_relative_position_of_money_relative_to_range(self, min_money, max_money, exchange)?;
        match relative_to_range {
            PositionRelativeToRange::BeforeRange => convert(exchange, &min_money, self.currency()),
            PositionRelativeToRange::WithinRange => Ok(self.clone()),
            PositionRelativeToRange::AfterRange => convert(exchange, &max_money, self.currency()),
        }
    }
}

impl<'a, T: FormattableCurrency> CurrencyIndependentAdd<'a, T> for Money<'a, T>{
    fn add(&self, other: &Money<'a, T>, output_currency: &T, exchange: &Exchange<'a, T>) -> Result<Money<'a, T>, ErrorCode> {
        let converted_self = convert(exchange, &self, output_currency)?;
        let converted_other = convert(exchange, &other, output_currency)?;
        
        Ok(converted_self + converted_other)
    }
}

impl<'a, T: FormattableCurrency> CurrencyIndependentSub<'a, T> for Money<'a, T>{
    fn sub(&self, other: &Money<'a, T>, output_currency: &T, exchange: &Exchange<'a, T>) -> Result<Money<'a, T>, ErrorCode> {
        let converted_self = convert(exchange, &self, output_currency)?;
        let converted_other = convert(exchange,&other, output_currency)?;

        Ok(converted_self - converted_other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_money::define_currency_set;
    use rust_decimal_macros::*;

    define_currency_set!(
        test {
            USD: {
                code: "USD",
                exponent: 2,
                locale: EnUs,
                minor_units: 100,
                name: "USD",
                symbol: "$",
                symbol_first: true,
            },
            GBP : {
                code: "GBP",
                exponent: 2,
                locale: EnUs,
                minor_units: 1,
                name: "British Pound",
                symbol: "£",
                symbol_first: true,
            }
        }
    );

    #[test]
    fn can_compare_usd_amount_with_greater_gbp_amount(){
        let usd = test::find("USD").unwrap();
        let gbp = test::find("GBP").unwrap();

        let usd_gbp_rate = ExchangeRate::new(usd, gbp, dec!(0.7)).unwrap();
        let mut exchange = Exchange::new(); 

        exchange.set_rate(&usd_gbp_rate);
        
        let usd_amount = Money::from_minor(2_00, test::USD);
        let gbp_amount = Money::from_minor(3_00, test::GBP);

        assert_eq!(usd_amount.currency_independent_lt(&gbp_amount, &exchange).unwrap(), true);
        assert_eq!(usd_amount.currency_independent_lte(&gbp_amount, &exchange).unwrap(), true);
        assert_eq!(usd_amount.currency_independent_gt(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_gte(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_eq(&gbp_amount, &exchange).unwrap(), false);

        assert_eq!(gbp_amount.currency_independent_lt(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_lte(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_gt(&usd_amount, &exchange).unwrap(), true);
        assert_eq!(gbp_amount.currency_independent_gte(&usd_amount, &exchange).unwrap(), true);
        assert_eq!(gbp_amount.currency_independent_eq(&usd_amount, &exchange).unwrap(), false);
    }

    #[test]
    fn can_compare_usd_amount_with_lesser_gbp_amount(){
        let usd = test::find("USD").unwrap();
        let gbp = test::find("GBP").unwrap();

        let usd_gbp_rate = ExchangeRate::new(usd, gbp, dec!(0.7)).unwrap();
        let mut exchange = Exchange::new(); 

        exchange.set_rate(&usd_gbp_rate);
        
        let usd_amount = Money::from_minor(5_00, test::USD);
        let gbp_amount = Money::from_minor(1_00, test::GBP);

        assert_eq!(usd_amount.currency_independent_lt(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_lte(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_gt(&gbp_amount, &exchange).unwrap(), true);
        assert_eq!(usd_amount.currency_independent_gte(&gbp_amount, &exchange).unwrap(), true);
        assert_eq!(usd_amount.currency_independent_eq(&gbp_amount, &exchange).unwrap(), false);

        assert_eq!(gbp_amount.currency_independent_lt(&usd_amount, &exchange).unwrap(), true);
        assert_eq!(gbp_amount.currency_independent_lte(&usd_amount, &exchange).unwrap(), true);
        assert_eq!(gbp_amount.currency_independent_gt(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_gte(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_eq(&usd_amount, &exchange).unwrap(), false);
    }

    #[test]
    fn can_compare_usd_amount_with_equal_gbp_amount(){
        let usd = test::find("USD").unwrap();
        let gbp = test::find("GBP").unwrap();

        let usd_gbp_rate = ExchangeRate::new(usd, gbp, dec!(0.7)).unwrap();
        let mut exchange = Exchange::new(); 

        exchange.set_rate(&usd_gbp_rate);
        
        let usd_amount = Money::from_minor(10_00, test::USD);
        let gbp_amount = Money::from_minor(7_00, test::GBP);

        assert_eq!(usd_amount.currency_independent_lt(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_lte(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_gt(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_gte(&gbp_amount, &exchange).unwrap(), false);
        assert_eq!(usd_amount.currency_independent_eq(&gbp_amount, &exchange).unwrap(), true);

        assert_eq!(gbp_amount.currency_independent_lt(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_lte(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_gt(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_gte(&usd_amount, &exchange).unwrap(), false);
        assert_eq!(gbp_amount.currency_independent_eq(&usd_amount, &exchange).unwrap(), true);
    }
}