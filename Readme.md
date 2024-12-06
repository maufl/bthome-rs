# BTHome in Rust
This repo contains a library for working with [BTHome](https://bthome.io/) data and a utility to sniff BTHome BLE advertisments on Linux.

To be able to use it, you have to enable [experimental features](https://wiki.archlinux.org/title/Bluetooth#Enabling_experimental_features) in bluez (I think).

## TODO
* Better API design
  * Is `Object` a good name? Should there be a distinction on type level between measurements, events and other?
  * Should I try to convert only to the smallest possible type? I.e. not use i64 for all integer values.
* Add serialization, so that I can use it in my other projects
* Make the library `no_std`, that would be nice.

Feedback and ideas very welcome :)