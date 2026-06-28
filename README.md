# wfmq

A command-line tool for querying [warframe.market](https://warframe.market) — find ducat-efficient trades, watch live sell orders, and check what's worth buying from a specific seller.

## Building from source

Requires the [Rust toolchain](https://www.rust-lang.org/tools/install) (stable).

```bash
git clone https://github.com/BenjaminBrienen/warframe-market-query.git
cd warframe-market-query
cargo build --release
```

The binary is then at `target/release/wfmq` (`target\release\wfmq.exe` on Windows).

While developing, you can skip the explicit build step and run subcommands straight through Cargo:

```bash
cargo run listen --ratio 25
cargo run user-ducats --ratio 25 --username ExampleUser
cargo run ducats --ratio 15 --quantity 6
```

## Using the prebuilt binaries

Prebuilt binaries are attached to every [release](https://github.com/BenjaminBrienen/warframe-market-query/releases/latest):

- **Linux:** [`wfmq`](https://github.com/BenjaminBrienen/warframe-market-query/releases/latest/download/wfmq)
- **Windows:** [`wfmq.exe`](https://github.com/BenjaminBrienen/warframe-market-query/releases/latest/download/wfmq.exe)

On Linux, mark the file executable before running it:

```bash
chmod +x wfmq
./wfmq listen --ratio 25
./wfmq user-ducats --ratio 25 --username ExampleUser
./wfmq ducats --ratio 15 --quantity 6
```

On Windows:

```powershell
.\wfmq.exe listen --ratio 25
.\wfmq.exe user-ducats --ratio 25 --username ExampleUser
.\wfmq.exe ducats --ratio 15 --quantity 6
```
