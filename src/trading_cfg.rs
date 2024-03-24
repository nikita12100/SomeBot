#[derive(Debug, Clone)]
pub struct HammerCfg {
    pub bottom_start: u8,
    // must be 0-100
    pub bottom_end: u8,
    // must be 0-100
    pub up_start: u8,
    // must be 0-100
    pub up_end: u8, // must be 0-100
}
#[derive(Debug, Clone)]
pub struct TrendCfg {
    pub max_candle_skip: i8
}

#[derive(Debug, Clone)]
pub struct HammerStrategySettings {
    pub hammer_cfg: HammerCfg,
    pub trend_cfg: TrendCfg,
    // как далеко мы смотрим назад при поиске паттерна при покупке
    pub window_size: u64, // in minutes
}

impl HammerCfg {
    pub fn new(
        bottom_start: u8,
        bottom_end: u8,
        up_start: u8,
        up_end: u8,
    ) -> Self {
        if bottom_start >= 0 && bottom_start <= 100 &&
            bottom_end >= 0 && bottom_end <= 100 &&
            up_start >= 0 && up_start <= 100 &&
            up_end >= 0 && up_end <= 100 &&
            bottom_start < bottom_end &&
            up_start < up_end {
            Self { bottom_start, bottom_end, up_start, up_end }
        } else {
            panic!("Incorrect HammerCfg")
        }
    }
}