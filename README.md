# lora-e5

[![CI](https://github.com/novalabsxyz/lora-e5/actions/workflows/rust.yml/badge.svg)](https://github.com/novalabsxyz/lora-e5/actions/workflows/rust.yml)

A Rust library for using the [LoRa E5](https://www.seeedstudio.com/LoRa-E5-Wireless-Module-p-4745.html) module with AT commands.

The LoRa E5 is available as a [chip module](https://www.seeedstudio.com/LoRa-E5-Wireless-Module-p-4745.html), or as a [ready-to-use USB device](https://www.seeedstudio.com/LoRa-E5-mini-STM32WLE5JC-p-4869.html).

## Hardware & Tests

This library has only been tested on the [LoRa E5 Dev board](https://www.seeedstudio.com/LoRa-E5-Dev-Kit-p-4868.html).

To run tests, plug in the board over USB, and run a single-threaded test:

```shell
cargo test --  --nocapture --test-threads 1
```
