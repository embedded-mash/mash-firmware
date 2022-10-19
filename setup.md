# ESP32 Setup

Setting the esp toolchain could be a bit tricky if dive into the espressif toolchain for the first time.

There are a view cornerstones you have to understand.

## System setup

To build the espressif project, you need some tools.

```bash
apt-get install -y git curl gcc clang ninja-build cmake libudev-dev unzip xz-utils python3 python3-pip python3-venv libusb-1.0-0 libssl-dev pkg-config libtinfo5 libpython2.7
```

Next, switch to the latest version of your toolchain `rustup install nightly` or propably update it.

```bash
git clone https://github.com/esp-rs/rust-build.git
cd rust-build
./install-rust-toolchain.sh
```

If you face any issues, please try to fix them first. I had to cleanup my rust installation completely.

With a bit of luck, this will finally end with:

```log
done
Removing cached dist files:
 - rust-1.63.0.2-x86_64-unknown-linux-gnu
 - rust-1.63.0.2-x86_64-unknown-linux-gnu.tar.xz
 - rust-src-1.63.0.2
 - rust-src-1.63.0.2.tar.xz
 - xtensa-esp32-elf-llvm14_0_0-esp-14.0.0-20220415-x86_64-unknown-linux-gnu.tar.xz
 - *-elf-gcc*.tar.gz
Add following command to /home/alex/.bashrc
export LIBCLANG_PATH="/home/alex/.espressif/tools/xtensa-esp32-elf-clang/esp-14.0.0-20220415-x86_64-unknown-linux-gnu/lib/"
export PATH="/home/alex/.espressif/tools/xtensa-esp32-elf-gcc/8_4_0-esp-2021r2-patch3-x86_64-unknown-linux-gnu/bin/:/home/alex/.espressif/tools/xtensa-esp32s2-elf-gcc/8_4_0-esp-2021r2-patch3-x86_64-unknown-linux-gnu/bin/:/home/alex/.espressif/tools/xtensa-esp32s3-elf-gcc/8_4_0-esp-2021r2-patch3-x86_64-unknown-linux-gnu/bin/:$PATH"
```

To get a fully flawlessly working environment, I would recommend you to add the environment variables to your terminal startup script. In my case i just add it to the `$HOME/.bashrc`

```bash
echo "" >> $HOME/.bashrc
echo export LIBCLANG_PATH="$HOME/.espressif/tools/xtensa-esp32-elf-clang/esp-14.0.0-20220415-x86_64-unknown-linux-gnu/lib/" >> $HOME/.bashrc
echo export PATH="$HOME/.espressif/tools/xtensa-esp32-elf-gcc/8_4_0-esp-2021r2-patch3-x86_64-unknown-linux-gnu/bin/:$HOME/.espressif/tools/xtensa-esp32s2-elf-gcc/8_4_0-esp-2021r2-patch3-x86_64-unknown-linux-gnu/bin/:$HOME/.espressif/tools/xtensa-esp32s3-elf-gcc/8_4_0-esp-2021r2-patch3-x86_64-unknown-linux-gnu/bin/:$PATH" >> $HOME/.bashrc
echo "" >> $HOME/.bashrc
```

AND, restart your terminal after this.

## Install tooling

To build the project and flash it to the device, you need two tools:

```bash
cargo install ldproxy
cargo install espflash
```

## Testing the setup

To verify your installation, we will download a demo project and try to build and flash it to a real device. In addition, we have to override toolchain in project before we can build it.

```bash
git clone https://github.com/ivmarkov/rust-esp32-std-demo demo
cd demo
rustup override set esp
```

The demo project requires

```bash
cargo build
```

In the case cargo build fails, make sure, that the environment is setup with the stuff we add to the `$HOME/.bashrc`.

After that, rerun it and best luck.

## Flash

There are two tools to flush the esp.

### `espflash`

Using `espflash`, will remove a lot of definition work form your shoulders. It *automatically* defines the flash-size and defines an appropriate partition-size. But you don't have the comfort af a automatic rebuild

```bash
espflash --speed 921600 /dev/ttyUSB0 ./target/xtensa-esp32-espidf/debug/rust-esp32-std-demo
```

### `cargo espflash`

Using `cargo espflash`, is a bit more powerful than `espflash` but you have to define a bit more to get the firmware running on the device. Frist, you have to setup the `./partitions.csv` and second you have to define the `flash-size` of your target.

You can use following tools to gether the required information.

- `cargo espflash board-info` shows the flash-size of your target
- `cargo espflash partition-table ./partitions.csv --info` to verify your partition setup.
  - Take care, that your last patition fit into the flash size (offset + size <= flash-size)

```bash
cargo espflash --speed 921600 --flash-size=4MB --partition-table=./partitions.csv /dev/ttyUSB0 --monitor
```
