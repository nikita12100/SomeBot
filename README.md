# SomeBot

## Usage

Set tokens in `utils/local_token.rs`, then `cargo run`

History training create folder `./hist_data`, then download zip file per (share, year), then unzip them into folder `[ticker]-[year]`, then remove zip file. Try it by run `cargo test`

## Road map

 - historical training[in progress]
 - hot config for strategy setting
 - stop loss
 - add hist load on start for prepare patterns
 - handle error while order
 - warm up problem
 - remove each unwrap for more stable work

Вопросы:
 - В исторических данных надо использовать `use mock_instant::SystemTime`, в песочнице и проде `std::time::SystemTime`.
 - выставлять заявки парой или нет?
 - нужен механизм очистки state от старрых данных
 - проверить арифметику на quotation

