# cloud-detect

![maintenance-status](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![crates-badge](https://img.shields.io/crates/v/cloud-detect.svg)](https://crates.io/crates/cloud-detect)
[![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

<p align="center">
  <img  src="assets/logo.png" alt="Pylon Logo">
</p>

Rust library that detects a host's cloud service provider.

This library is heavily inspired by the Python [cloud-detect](https://github.com/dgzlopes/cloud-detect)
module, and replicates most of its functionality (even most of the code is structured similarly).

## Features

* Supports the identification of the following providers:
    - Amazon Web Services (`aws`)
    - Microsoft Azure (`azure`)
    - Google Cloud Platform (`gcp`)
    - Alibaba Cloud (`alibaba`)
    - OpenStack (`openstack`)
    - Oracle Cloud Infrastructure (`oci`)
    - Vultr (`vultr`)
* Fast, simple and extensible.
* Real-time logging in the console.

## Requirements

### Linux

OpenSSL 1.0.1, 1.0.2, 1.1.0, or 1.1.1 with headers (see https://github.com/sfackler/rust-openssl)

#### Ubuntu and Debian-based distributions

```bash
$ sudo apt-get install pkg-config libssl-dev
```

#### Fedora and RHEL-based distributions

```bash
$ sudo dnf install pkg-config perl-FindBin openssl-devel
```

OR

```bash
$ sudo yum install pkg-config perl-FindBin openssl-devel
```

#### SLES and openSUSE

```bash
$ sudo zypper in pkg-config libopenssl-devel
```

#### Arch Linux

```bash
$ sudo pacman -S pkg-config openssl
```

## Usage

First, add the library to your project by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
cloud-detect = "1.0.0"
tokio = { version = "1.29.1", features = ["full"] }
tracing-subscriber = "0.3.17" # Only needed if real-time logging is required.
```

Next, you can detect the current host's cloud provider as follows:

```rust
use cloud_detect::detect;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init(); // Only needed if real-time logging is required.

    // With default timeout (5 seconds).
    let provider = detect(None).await;
    // With custom timeout.
    let provider = detect(Some(1)).await; // 1 second.

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

You can also check the list of currently supported cloud providers.

```rust
use cloud_detect::SUPPORTED_PROVIDERS;

#[tokio::main]
async fn main() {
    println!("{}", SUPPORTED_PROVIDERS.join(", "));
}
```

**NOTE**: Currently, only asynchronous detection is supported. Blocking detection *may* be added to a future release.

## Contributing

TODO