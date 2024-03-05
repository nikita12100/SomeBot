pub trait State<Event> {
    fn new() -> Self;
    fn update(&mut self, event: Event) -> Result<(), &'static str>;
}