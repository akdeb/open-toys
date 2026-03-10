# Elato Local AI

## Installation instructions

1. Clone the repository with `git clone https://github.com/akdeb/elato-local.git`
2. Install Rust and Tauri with `curl https://sh.rustup.rs -sSf | sh`
3. Install Node from [here](https://nodejs.org/en/download)
4. Run `cd app`
5. Run `npm install`
6. Run `npm run tauri dev`

## Flash to ESP32

1. Go to `AI Settings` and click on `Flash Firmware` with your ESP32-S3 device connected to your MacOS Apple Silicon device.
2. The device will open a WiFi captive portal `ELATO` to configure the WiFi network.
3. Add your WiFi network details and click connect.
4. Make sure your MacOS is on the same WiFi network. 
5. Your ESP32 should now connect whenever it is powered on while the server is running!

## Tested on

1. M1 Pro 2021 Macbook Pro
2. M4 Pro 2024 Macbook Pro

## Project Structure

```
elato/
├── app/
├── arduino/
├── resources/
├────────── python-backend/
├────────── firmware/
└── README.md
```

Python 3.11 runtime is downloaded on first app setup into the app data directory.
