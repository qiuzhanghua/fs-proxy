# FS Proxy
A simple file system proxy server built with Rust and Salvo framework.
It allows you to perform basic file operations over HTTP, such as reading, writing, and listing files in a specified directory.

## Features
- Read files from the server
- Write files to the server
- List files in a directory
- Asynchronous handling of requests using Tokio
- Structured logging with Tracing

## Requirements
- Rust (latest stable version recommended)
- Cargo (comes with Rust)
- Tokio
- Salvo framework
- Tracing for logging

## DB Support
```bash
cargo install sqlx-cli
```
and
```
mkdir data
```

1. SQLite
    ```bash
    # rm data/fs-proxy.sqlite
    cd db/sqlite
    export DATABASE_URL=sqlite:../../data/fs-proxy.sqlite
    sqlx db create
    sqlx migrate run
    cd ../..
    ```



## Usage
1. Clone the repository:
   ```bash
   git clone git@github.com:qiuzhanghua/fs-proxy.git
    cd fs-proxy
    ```
2. Build the project:
   ```bash
   cargo build --release
   ```
3. Run the server:
    ```bash
    cargo run --release
    ```
4. Access the server at `http://localhost:8080`.
