use prost_types::Timestamp;
use tinkoff_invest_api::tcs::Quotation;

pub trait Cmp<T> {
    fn _le(&self, other: &T) -> bool;
    fn _leq(&self, other: &T) -> bool;
    fn _ge(&self, other: &T) -> bool;
    fn _geq(&self, other: &T) -> bool;
}

impl Cmp<Timestamp> for Timestamp {
    fn _le(&self, other: &Timestamp) -> bool {
        self.seconds < other.seconds ||
            (self.seconds == other.seconds && self.nanos < other.nanos)
    }

    fn _leq(&self, other: &Timestamp) -> bool {
        self.seconds < other.seconds ||
            (self.seconds == other.seconds && self.nanos < other.nanos) ||
            self == other
    }

    fn _ge(&self, other: &Timestamp) -> bool {
        self.seconds > other.seconds ||
            (self.seconds == other.seconds && self.nanos > other.nanos)
    }

    fn _geq(&self, other: &Timestamp) -> bool {
        self.seconds > other.seconds ||
            (self.seconds == other.seconds && self.nanos > other.nanos) &&
                self == other
    }
}
