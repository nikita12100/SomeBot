use tinkoff_invest_api::tcs::Candle;
use crate::utils::quotation::QuotationExtension;

pub trait CandleExtension {
    // бычья свеча, цена закрытия выше цены открытия
    fn is_bullish(&self) -> bool;
    // медвежбя свеча, цена закрытия нижу цены открытия
    fn is_bearish(&self) -> bool;
    fn percentage_open(&self) -> u8;
    fn percentage_close(&self) -> u8;
}

impl CandleExtension for Candle {
    fn is_bullish(&self) -> bool {
        self.close.clone().unwrap().wr() > self.open.clone().unwrap().wr()
    }

    fn is_bearish(&self) -> bool {
        self.open.clone().unwrap().wr() > self.close.clone().unwrap().wr()
    }

    fn percentage_open(&self) -> u8 {
        let open = self.open.clone().unwrap().to_f();
        let high = self.high.clone().unwrap().to_f();
        let low = self.low.clone().unwrap().to_f();

        let open_prc = (((open - low) / (high - low)) * 100.0).round() as u8;

        if open_prc < 0 {
            panic!("open_prc={}<0 for candle={:#?}", open_prc, self);
        }
        if open_prc > 100 {
            panic!("open_prc={}>100 for candle={:#?}", open_prc, self);
        }
        open_prc
    }

    fn percentage_close(&self) -> u8 {
        let close = self.close.clone().unwrap().to_f();
        let high = self.high.clone().unwrap().to_f();
        let low = self.low.clone().unwrap().to_f();

        let close_prc = (((close - low) / (high - low)) * 100.0).round() as u8;

        if close_prc < 0 {
            eprint!("close_prc={}<0 for candle={:#?}", close_prc, self);
        }
        if close_prc > 100 {
            eprint!("close_prc={}>100 for candle={:#?}", close_prc, self);
        }
        close_prc
    }
}