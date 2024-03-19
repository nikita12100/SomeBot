# SomeBot

## Usage

Set tokens in `local_token.rs`, then `cargo run`

History training create folder `./hist_data`, then download zip file per (share, year), then unzip them into folder `[ticker]-[year]`, then remove zip file. Try it by run `cargo test`

## Road map

 - historical training
 - handle error while order
 - warm up problem
 - remove each unwrap for more stable work

