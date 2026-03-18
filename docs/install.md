# Install rtfkit

The recommended way to install `rtfkit` is to download a prebuilt binary from [GitHub Releases](https://github.com/TorstenCScholz/rtfkit/releases).

## Release artifacts

Stable releases currently ship the following binary archives:

- `rtfkit-x86_64-unknown-linux-gnu.tar.gz`
- `rtfkit-aarch64-unknown-linux-gnu.tar.gz`
- `rtfkit-x86_64-apple-darwin.tar.gz`
- `rtfkit-aarch64-apple-darwin.tar.gz`
- `rtfkit-x86_64-pc-windows-msvc.zip`

Each release also includes per-archive SHA256 checksum files.

## macOS and Linux

1. Download the archive for your platform from GitHub Releases.
2. Extract the archive:

   ```sh
   tar xzf rtfkit-<target>.tar.gz
   ```

3. Move the binary to a directory on your `PATH`, for example:

   ```sh
   install -m 755 rtfkit /usr/local/bin/rtfkit
   ```

4. Verify the installation:

   ```sh
   rtfkit --help
   ```

## Windows

1. Download `rtfkit-x86_64-pc-windows-msvc.zip` from GitHub Releases.
2. Extract `rtfkit.exe`.
3. Move `rtfkit.exe` into a directory on your `PATH`, or add its directory to `PATH`.
4. Verify the installation:

   ```powershell
   rtfkit.exe --help
   ```

## Verify checksums

### macOS and Linux

Download the archive and matching `.sha256` file, then run:

```sh
sha256sum -c rtfkit-<target>.tar.gz.sha256
```

### Windows

Use PowerShell to compute the file hash and compare it with the published checksum:

```powershell
Get-FileHash .\rtfkit-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

## Build from source

If you need a local development build instead of a release artifact:

```sh
cargo install --path crates/rtfkit-cli
```
