use prost_types::Timestamp;

#[derive(Debug, Clone)]
pub struct WrapperMockSystemTime(pub mock_instant::SystemTime);

impl From<WrapperMockSystemTime> for Timestamp {
    fn from(system_time: WrapperMockSystemTime) -> Timestamp {
        let (seconds, nanos) = match system_time.0.duration_since(mock_instant::UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                (seconds, duration.subsec_nanos() as i32)
            }
            Err(error) => {
                let duration = error.duration();
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                let nanos = duration.subsec_nanos() as i32;
                if nanos == 0 {
                    (-seconds, 0)
                } else {
                    (-seconds - 1, 1_000_000_000 - nanos)
                }
            }
        };
        Timestamp { seconds, nanos }
    }
}