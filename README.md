## IKEA Sparsnäs decoder

Small library to decode packets transmitted by the IKEA Sparsnäs
energy monitor.

This is based on the impressive (and well documented) reverse
engineering work done by "kodarn": https://github.com/kodarn/Sparsnas

## Intended usage

```rust
// Decoder for transmitter with serial 400-565-321
let d = SparsnasDecoder::new(400_565_321);

let pktbuf = get_packet_from_radio();
let pkt = d.decode(pktbuf)?;
// Meter emits 1000 blinks/kWh
println!("Power: {}", pkt.power(1000));
```

## License

Licensed at your option under either of:

* [MIT License](LICENSE-MIT)
* [Apache License, Version 2.0](LICENSE-APACHE)
