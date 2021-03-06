use rust_decimal::prelude::*;
use std::collections::HashMap;
use std::cmp::Ordering;
use simple_money::*;
use rust_decimal_macros::*;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TaxError {
    #[error("Mismatched currencies")]
    MismatchedCurrencies,
    #[error("Could not find deduction")]
    CouldNotFindDeduction,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct TaxBracket{
    min_money: Money,
    max_money: Option<Money>,
    rate: Decimal,
}

impl PartialOrd for TaxBracket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.min_money.partial_cmp(&other.min_money)
    }
}

impl Ord for TaxBracket {
    fn cmp(&self, other: &Self) -> Ordering {
        self.min_money.cmp(&other.min_money)
    }
}

impl TaxBracket {
    fn new_tax_bracket_with_max(
        min_money: Money,
        max_money: Money,
        rate: Decimal,
    ) -> Result<TaxBracket, TaxError> {
        if min_money.currency != max_money.currency {
            Err(TaxError::MismatchedCurrencies)
        }else{
            Ok(TaxBracket{
                min_money,
                max_money: Some(max_money),
                rate,
            })
        }
    }

    pub fn new(
        min_money: Money,
        max_money: Option<Money>,
        rate: Decimal,
    ) -> Result<TaxBracket, TaxError> {
        if let Some(max_money) = max_money {
            Self::new_tax_bracket_with_max(min_money, max_money, rate)
        }else{
            Ok(TaxBracket{ min_money, max_money: None, rate})
        }
    }

    pub fn calculate_tax(&self, taxable_income: Money) -> Money {
        if taxable_income < self.min_money {
            return Money { amount: dec!(0), currency: self.min_money.currency };
        }

        if let Some(max_money) = self.max_money {
            if taxable_income >= max_money {
                return max_money * self.rate;
            }else{
                return (taxable_income - self.min_money) * self.rate;
            }
        }

        return (taxable_income - self.min_money) * self.rate;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaxDeductionCategory {
    CapitalGains,
    EmployeeStockOptions,
}

#[derive(Clone, Copy, Debug)]
pub struct TaxDeductionRule {
    pub tax_deduction_type: TaxDeductionCategory,
    pub max_amount: Option<Money>,
    pub inclusion_rate: Decimal,
}

impl TaxDeductionRule {
    pub fn apply_deduction(&self, deduction: TaxDeduction) -> Money {
        if let Some(max_amount) = self.max_amount {
            if deduction.money_to_deduct <= max_amount {
                return max_amount * self.inclusion_rate
            }else{
                return deduction.money_to_deduct * self.inclusion_rate
            }
        }

        return deduction.money_to_deduct * self.inclusion_rate;
    }
}

pub struct TaxDeduction {
    pub tax_deduction_type: TaxDeductionCategory,
    pub money_to_deduct: Money,
}

#[derive(Debug, Clone)]
pub struct TaxSchedule {
    brackets: Vec<TaxBracket>,
    deductions_map: HashMap<TaxDeductionCategory, TaxDeductionRule>,
    tax_currency: Currency,
}

impl TaxSchedule {
    fn validate_currency_on_bracket(bracket: &TaxBracket, currency: Currency) -> bool {
       if let Some(max_money) = bracket.max_money{
           max_money.currency == currency && bracket.min_money.currency == currency
       }else{
           bracket.min_money.currency == currency
       }
    }

    fn validate_currency_on_brackets(brackets: Vec<TaxBracket>, currency: Currency) -> bool {
        brackets.iter().all(|bracket| Self::validate_currency_on_bracket(bracket, currency))
    }

    pub fn new(
        brackets: Vec<TaxBracket>,
        currency: Currency,
    ) -> Result<TaxSchedule, TaxError> {
        if !Self::validate_currency_on_brackets(brackets.clone(), currency){
           Err(TaxError::MismatchedCurrencies) 
        }else{
            let mut new_brackets = brackets.clone();
            new_brackets.sort();
            return Ok(TaxSchedule {
                brackets: new_brackets,
                deductions_map: HashMap::new(),
                tax_currency: currency,
            })
        }
    }

    pub fn set_deduction(
        &mut self,
        tax_deduction_category: TaxDeductionCategory,
        tax_deduction_rule: TaxDeductionRule,
    ){
        self.deductions_map.insert(tax_deduction_category, tax_deduction_rule);
    }

    fn determine_deductions_amount(
        &self,
        deductions: Vec<TaxDeduction>,
    ) -> Result<Money, TaxError> {
        deductions
            .iter()
            .try_fold( Money { amount: dec!(0), currency: self.tax_currency } , |acc, actual_tax_deduction| {
                match self
                    .deductions_map
                    .get(&actual_tax_deduction.tax_deduction_type)
                {
                    Some(deduction_info) => {
                        let money_result = actual_tax_deduction.money_to_deduct
                            * deduction_info.inclusion_rate
                            + acc;
                        Ok(money_result)
                    }
                    None => Err(TaxError::CouldNotFindDeduction),
                }
            })
    }

    pub fn calculate_tax(&self, taxable_income: Money) -> Money {
        self.brackets
            .iter()
            .map(|bracket| bracket.calculate_tax(taxable_income.clone()))
            .fold(Money { amount: dec!(0), currency: taxable_income.currency }, |acc, bracket_tax| acc + bracket_tax)
    }

    pub fn calculate_tax_with_deductions(
        &self,
        income: Money,
        deductions: Vec<TaxDeduction>,
    ) -> Result<Money, TaxError> {
        let deductions_amount = self.determine_deductions_amount(deductions);
        match deductions_amount {
            Ok(deductions_total) => Ok(self.calculate_tax(income - deductions_total)),
            Err(error_code) => Err(error_code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_example() {
        let lowest = TaxBracket {
            min_money: cad_money!(0),
            max_money: Some(cad_money!(10_000)),
            rate: dec!(0.1),
        };
        let middle = TaxBracket {
            min_money: cad_money!(10_000),
            max_money: Some(cad_money!(20_000)),
            rate: dec!(0.2),
        };
        let highest = TaxBracket {
            min_money: cad_money!(20_000),
            max_money: None,
            rate: dec!(0.3),
        };

        let schedule = TaxSchedule::new(vec![lowest, middle, highest], Currency::CAD).unwrap();

        let over_highest_tax = schedule.calculate_tax(cad_money!(25_000));
        assert_eq!(over_highest_tax, cad_money!(6_500));

        let middle_tax = schedule.calculate_tax(cad_money!(15_000));
        assert_eq!(middle_tax, cad_money!(2000));

        let lowest_tax = schedule.calculate_tax(cad_money!(5_000));
        assert_eq!(lowest_tax, cad_money!(500));
    }

    #[test]
    fn single_bracket_example() {
        let lowest = TaxBracket {
            min_money: cad_money!(0),
            max_money: Some(cad_money!(10_000)),
            rate: dec!(0.1),
        };

        let schedule = TaxSchedule::new(vec![lowest], Currency::CAD).unwrap();
        let tax = schedule.calculate_tax(cad_money!(10_000));

        assert_eq!(tax, cad_money!(1000));
    }

    #[test]
    fn invalid_bracket_and_regime(){
        let invalid = TaxBracket::new(
            cad_money!(0), 
            Some(usd_money!(1)), 
            dec!(0.1)
        ).unwrap_err();

        assert_eq!(invalid, TaxError::MismatchedCurrencies);

        let valid_bracket = TaxBracket::new(
            cad_money!(0),
            None,
            dec!(0.1)
        ).unwrap();
        let invalid_schedule = TaxSchedule::new(
            vec![valid_bracket],
            Currency::USD,
        ).unwrap_err();

        assert_eq!(invalid_schedule, TaxError::MismatchedCurrencies);
    }

    #[test]
    fn deductions_example() {
        let single = TaxBracket {
            min_money: cad_money!(0),
            max_money: None,
            rate: dec!(0.1),
        };
        let capital_gains_deduction = TaxDeductionRule {
            tax_deduction_type: TaxDeductionCategory::CapitalGains,
            max_amount: None,
            inclusion_rate: dec!(0.5),
        };

        let mut schedule = TaxSchedule::new(
            vec![single],
            Currency::CAD,
        ).unwrap();
        schedule.set_deduction(
            TaxDeductionCategory::CapitalGains, 
            capital_gains_deduction
        );
        let actual_deductions = vec![TaxDeduction {
            tax_deduction_type: TaxDeductionCategory::CapitalGains,
            money_to_deduct: cad_money!(5000),
        }];
        let tax = schedule.calculate_tax_with_deductions(cad_money!(10_000), actual_deductions);

        match tax {
            Ok(result) => assert_eq!(result, cad_money!(750.00)),
            Err(_) => assert!(false, "Tax should not be an Err"),
        }
    }
}