use std::cmp::Ordering;
use std::ops::{Add, Mul, Sub};
use std::str::FromStr;
use tinkoff_invest_api::tcs::Quotation;

// https://russianinvestments.github.io/investAPI/faq_custom_types/
#[derive(Debug, Clone)]
pub struct QuotationWrapper {
    q: Quotation,
}

pub trait QuotationExtension {
    fn wr(&self) -> QuotationWrapper;
    fn to_f(&self) -> f64;
    fn from_str(str: &str) -> Quotation;
}

impl QuotationExtension for Quotation {
    fn wr(&self) -> QuotationWrapper {
        QuotationWrapper { q: self.clone() }
    }

    fn to_f(&self) -> f64 {
        self.units as f64 + self.nano as f64 / f64::powi(10.0, 9)
    }
    fn from_str(str: &str) -> Quotation {
        let f: f64 = str.parse().unwrap();
        Quotation {
            units: f.trunc() as i64,
            nano: (f.fract() * 1000000000.0) as i32,
        }
    }
}


// ------------------------------------------- < > = -------------------------------------------
impl Eq for QuotationWrapper {}

impl PartialEq for QuotationWrapper {
    fn eq(&self, other: &Self) -> bool {
        (self.q.units, &self.q.nano) == (other.q.units, &other.q.nano)
    }
}

impl PartialOrd<Self> for QuotationWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QuotationWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.q.units < other.q.units ||
            (self.q.units == other.q.units && self.q.nano < other.q.nano) {
            Ordering::Less
        } else if self.q.units > other.q.units ||
            (self.q.units == other.q.units && self.q.nano > other.q.nano) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

// ------------------------------------------- + - * -------------------------------------------


impl Add for QuotationWrapper {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let (nano, carry) = match self.q.nano.checked_add(other.q.nano) {
            Some(summ) => (summ, 0_i64),
            None => {
                let mut nano_i64 = self.q.nano as i64 + other.q.nano as i64;
                nano_i64 -= i32::MAX as i64;
                (nano_i64 as i32, 1_i64)
            }
        };
        let units = self.q.units + other.q.units + carry;
        QuotationWrapper {
            q: Quotation {
                units: units,
                nano: nano,
            },
        }
    }
}

impl Sub for QuotationWrapper {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let (nano, carry) = match self.q.nano.checked_sub(other.q.nano) {
            Some(sub) => (sub, 0_i64),
            None => {
                (self.q.nano + (i32::MAX - other.q.nano) + i32::MAX, 1_i64)
            }
        };
        let units = self.q.units - other.q.units - carry;
        QuotationWrapper {
            q: Quotation {
                units: units,
                nano: nano,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use tinkoff_invest_api::tcs::Quotation;
    use crate::utils::quotation::QuotationExtension;

    #[test]
    fn to_f_test() {
        let q1 = Quotation { units: 114, nano: 250000000 };
        let f1 = 114.25;
        assert_eq!(q1.to_f(), f1);
        assert_eq!(<Quotation as QuotationExtension>::from_str(&f1.to_string()), q1);

        let q2 = Quotation { units: -200, nano: -200000000 };
        let f2 = -200.20;
        assert_eq!(q2.to_f(), f2);
        // assert_eq!(<Quotation as QuotationExtension>::from_str(&f2.to_string()), q2);

        let q3 = Quotation { units: -0, nano: -10000000 };
        let f3 = -0.01;
        assert_eq!(q3.to_f(), f3);
        assert_eq!(<Quotation as QuotationExtension>::from_str(&f3.to_string()), q3);
    }

    #[test]
    fn test_arith() {
        let x_1 = Quotation { units: 114, nano: i32::MAX };
        let y_1 = Quotation { units: 6, nano: 1 };
        let add = Quotation { units: 121, nano: 1 };
        assert_eq!(x_1.wr() + y_1.wr(), add.wr());

        let x_2 = Quotation { units: 114, nano: i32::MIN };
        let y_2 = Quotation { units: 6, nano: 1 };
        let sub = Quotation { units: 107, nano: 2147483645 };
        assert_eq!(x_2.wr() - y_2.wr(), sub.wr());
    }
}


