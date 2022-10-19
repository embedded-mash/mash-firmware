# mesh-firmware

The purpose of this project is to build an self optimizing mesh of ESP32 MPUs with WiFi.

Some major targets of this project are:

- Optimization should interrupt the communication as minimal as possible
- Every node should act as AP for other clients (PC, Mobile, WiFi-Routers)
- The optimization should be configurable as Least Hops / Best connection
- (TBD)

The second part of this project is the try to implement an data architecture to replicate
information between a huge number of nodes with very minimal amount of traffic.

## Setup

The ESP toolchain is not included in the release toolchain (for now).
Follow the instructions [here](./setup.md) to prepare your local setup to build for the esp.

## Flash

To flash your firmware to the ESP-32 use:

```bash
cargo espflash --speed 921600 --flash-size=4MB --partition-table=./partitions.csv /dev/ttyUSB0 --monitor
```
