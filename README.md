# cloud-detect

![maintenance-status](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![crates-badge](https://img.shields.io/crates/v/cloud-detect.svg)](https://crates.io/crates/cloud-detect)
[![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

A Rust library to detect the cloud service provider of a host.

This library is inspired by the Python-based [cloud-detect](https://github.com/dgzlopes/cloud-detect)
and the Go-based [satellite](https://github.com/banzaicloud/satellite) modules.

Like these modules, `cloud-detect` uses a combination of checking vendor files and metadata endpoints to accurately
determine the cloud provider of a host.

## Features

* Currently, this module supports the identification of the following providers:
    - Amazon Web Services (`aws`)
    - Microsoft Azure (`azure`)
    - Google Cloud Platform (`gcp`)
    - Alibaba Cloud (`alibaba`)
    - OpenStack (`openstack`)
    - DigitalOcean (`digitalocean`)
    - Oracle Cloud Infrastructure (`oci`)
    - Vultr (`vultr`)
* Fast, simple and extensible.
* Real-time console logging using the [`tracing`](https://crates.io/crates/tracing) crate.

## Usage

First, add the library to your project by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
# ...
cloud-detect = "1"
tokio = { version = "1", features = ["full"] }
tracing-subscriber = { version = "0.2", features = ["env-filter"] }# Optional; for logging.
```

Detect the cloud provider and print the result (with default timeout).

```rust
use cloud_detect::detect;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init(); // Optional; for logging

    let provider = detect(None).await;

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

Detect the cloud provider and print the result (with custom timeout).

```rust
use cloud_detect::detect;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init(); // Optional; for logging

    let provider = detect(Some(10)).await;

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

You can also check the list of currently supported cloud providers.

```rust
use cloud_detect::supported_providers;

#[tokio::main]
async fn main() {
    println!("Supported providers: {:?}", supported_providers.await);
}
```

For more detailed documentation, please refer to the [Crate Documentation](https://docs.rs/cloud-detect).

## Contributing

Contributions are welcome and greatly appreciated! If you’d like to contribute to cloud-detect, here’s how you can help.

### 1. Report Issues

If you encounter a bug, unexpected behavior, or have a feature request, please open
an [issue](https://github.com/nikhil-prabhu/cloud-detect/issues/new).
Be sure to include:

* A clear description of the issue.
* Steps to reproduce, if applicable.
* Details about your environment.

### 2. Submit Pull Requests

If you're submitting a [pull request](https://github.com/nikhil-prabhu/cloud-detect/compare), please ensure the
following.

* Your code is formatted using `cargo fmt` (the Rust `nightly` channel is required, as a few unstable features are
  used).

```bash
$ cargo fmt +nightly --all
$ cargo fmt +nightly --all --check
```

* Code lints pass with:

```bash
$ cargo clippy --all-targets --all-features --workspace -- -D warnings
````

* Your code contains sufficient unit tests and that all tests pass.

```bash
$ cargo test --locked --all-features --workspace
```

### 3. Improve Documentation

If you find areas in the documentation that are unclear or incomplete, feel free to update the README or crate-level
documentation. Open a pull request with your improvements.

### 4. Review Pull Requests

You can also contribute by
reviewing [open pull requests](https://github.com/nikhil-prabhu/cloud-detect/pulls?q=is%3Aopen+is%3Apr). Providing
constructive feedback helps maintain a
high-quality
codebase.
