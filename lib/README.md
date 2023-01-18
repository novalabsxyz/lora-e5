# lora-e5

[![CI](https://github.com/novalabsxyz/lora-e5/actions/workflows/rust.yml/badge.svg)](https://github.com/novalabsxyz/lora-e5/actions/workflows/rust.yml)

Rust library for using the LoRa E5 with AT commands.

## Hardware & Tests

This library has only been tested on the LoRa E5 Dev board.

To run tests, plug the board over USB and run a single-threaded test:

```shell
cargo test --  --nocapture --test-threads 1
```
