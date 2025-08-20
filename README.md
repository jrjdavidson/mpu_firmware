# MPU-6050 BLE Telemetry Firmware

Firmware in 100 percent Rust (`no_std`) for the ESP32-C6.  
It samples six-degree-of-freedom motion data from an MPU-6050 over I²C, packetises it, and streams it via Bluetooth Low Energy notifications so a phone or PC can plot live orientation and acceleration. This firmware is designed to be run with the motion_ble webpage.

Repo: `jrjdavidson/mpu_firmware`

---

## 1. Hardware

| Piece | Tested Part No.        | Notes                  |
|-------|------------------------|------------------------|
| MCU   | ESP32-C6               | RISC-V core, BLE 5.0   |
| IMU   | MPU-6050 (GY-521 board)| 3.3 V tolerant         |

---

## 2. Quick start

> The first build on Windows can take 10-15 minutes while LLVM builds the core crate. Assumes that git is installed, VSCode is recommended for development.

### 2.1 Install Rust

Follow the official guide: <https://www.rust-lang.org/tools/install>

Verify:

```powershell
rustup -V
```

### 2.2 Set up toolchain

```powershell
# components and target
rustup component add rust-src llvm-tools-preview
rustup target add riscv32imac-esp-espidf

# esp-rs bootstrap
# haven't tried this myself ,not sure if it will work
cargo install espup --locked
espup install --export-file esp.env

# probe-rs tools for on-chip debugging
irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex
```

### 2.3 Build, flash, and monitor

```powershell
git clone https://github.com/jrjdavidson/mpu_firmware
cd mpu_firmware

# auto-detects the ESP32-C6
cargo run -r
# or use the VS Code debugger with probe-rs (Ctrl-Shift-d)
```

---

## 3. BLE output 

Can be read with the motion reporter website, which uses WebBluetooth to stream the data to a graph.


---

## 4. Further recommmended reading

- esp-rs book: <https://esp-rs.github.io/book>  
- MPU-6050 register map: <https://www.invensense.com/wp-content/uploads/2015/02/MPU-6000-Register-Map1.pdf>  
- BLE GATT design guide: <https://developer.bluetooth.org/gatt>
- MPU-6050-dmp repo: <https://github.com/barafael/mpu6050-dmp-rs>

---