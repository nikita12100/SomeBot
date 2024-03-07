pub trait State<Event> {
    fn new() -> Self;
    fn update(&self, event: &Event) -> Result<(), Box<dyn std::error::Error>>;
}
